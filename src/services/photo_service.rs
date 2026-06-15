use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use kubuno_storage::StorageBackend;

use crate::models::{ListPhotosQuery, Photo, UpdatePhotoDto};

/// Liste les photos d'un utilisateur.
pub async fn list_photos(
    db: &PgPool,
    owner_id: Uuid,
    q: ListPhotosQuery,
) -> Result<Vec<Photo>> {
    let limit  = q.limit.unwrap_or(100).min(500);
    let offset = q.offset.unwrap_or(0);

    let photos = if q.trashed == Some(true) {
        sqlx::query_as::<_, Photo>(
            r#"SELECT p.*,
                (SELECT COUNT(*) FROM photos.album_photos ap WHERE ap.photo_id = p.id) AS _unused
               FROM photos.photos p
               WHERE p.owner_id = $1 AND p.is_trashed = TRUE
               ORDER BY p.trashed_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(owner_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .context("list_photos trashed")?
    } else if q.starred == Some(true) {
        sqlx::query_as::<_, Photo>(
            r#"SELECT * FROM photos.photos
               WHERE owner_id = $1 AND is_starred = TRUE AND is_trashed = FALSE
               ORDER BY taken_at DESC NULLS LAST, created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(owner_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .context("list_photos starred")?
    } else if let Some(album_id) = q.album_id {
        sqlx::query_as::<_, Photo>(
            r#"SELECT p.* FROM photos.photos p
               INNER JOIN photos.album_photos ap ON ap.photo_id = p.id
               WHERE ap.album_id = $1 AND p.owner_id = $2 AND p.is_trashed = FALSE
               ORDER BY ap.added_at DESC
               LIMIT $3 OFFSET $4"#,
        )
        .bind(album_id)
        .bind(owner_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .context("list_photos by album")?
    } else {
        sqlx::query_as::<_, Photo>(
            r#"SELECT * FROM photos.photos
               WHERE owner_id = $1
                 AND is_trashed = FALSE
                 AND ($3::timestamptz IS NULL OR taken_at >= $3)
                 AND ($4::timestamptz IS NULL OR taken_at <= $4)
                 AND ($5::text IS NULL OR original_name ILIKE '%' || $5 || '%' OR description ILIKE '%' || $5 || '%')
               ORDER BY taken_at DESC NULLS LAST, created_at DESC
               LIMIT $2 OFFSET $6"#,
        )
        .bind(owner_id)
        .bind(limit)
        .bind(q.from)
        .bind(q.to)
        .bind(q.search.as_deref())
        .bind(offset)
        .fetch_all(db)
        .await
        .context("list_photos")?
    };

    Ok(photos)
}

/// Récupère une photo par ID (vérifie ownership).
pub async fn get_photo(db: &PgPool, id: Uuid, owner_id: Uuid) -> Result<Option<Photo>> {
    let photo = sqlx::query_as::<_, Photo>(
        "SELECT * FROM photos.photos WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .context("get_photo")?;

    Ok(photo)
}

/// Upload et enregistre une photo.
pub async fn upload_photo(
    db: &PgPool,
    storage: &dyn StorageBackend,
    owner_id: Uuid,
    original_name: &str,
    data: Bytes,
    max_bytes: u64,
    thumbnail_size: u32,
    preview_size: u32,
) -> anyhow::Result<Photo> {
    if data.len() as u64 > max_bytes {
        anyhow::bail!("FILE_TOO_LARGE");
    }

    let mime = mime_guess::from_path(original_name)
        .first_or_octet_stream()
        .to_string();

    // Seuls les formats image sont acceptés
    if !is_image_mime(&mime) {
        anyhow::bail!("UNSUPPORTED_FORMAT");
    }

    let hash = hex::encode(Sha256::digest(&data));

    // Dimensions + EXIF via image crate
    let (width, height, taken_at, camera_make, camera_model, gps_lat, gps_lon) =
        extract_metadata(&data);

    let id           = Uuid::new_v4();
    let ext          = std::path::Path::new(original_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");
    let sanitized    = sanitize_filename::sanitize(original_name);
    let storage_path = format!("photos/{owner_id}/{id}.{ext}");

    storage.put(&storage_path, data.clone()).await
        .context("Stockage de la photo")?;

    // Générer thumbnail et preview
    let (has_thumbnail, has_preview) = generate_derivatives(
        storage, owner_id, id, &data, thumbnail_size, preview_size,
    ).await;

    let photo = sqlx::query_as::<_, Photo>(
        r#"INSERT INTO photos.photos
           (id, owner_id, filename, original_name, mime_type, size_bytes, width, height,
            storage_path, content_hash, taken_at, camera_make, camera_model,
            gps_lat, gps_lon, has_thumbnail, has_preview)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)
           RETURNING *"#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(&sanitized)
    .bind(original_name)
    .bind(&mime)
    .bind(data.len() as i64)
    .bind(width)
    .bind(height)
    .bind(&storage_path)
    .bind(&hash)
    .bind(taken_at)
    .bind(camera_make.as_deref())
    .bind(camera_model.as_deref())
    .bind(gps_lat)
    .bind(gps_lon)
    .bind(has_thumbnail)
    .bind(has_preview)
    .fetch_one(db)
    .await
    .context("Insertion photo en DB")?;

    Ok(photo)
}

/// Met à jour les métadonnées d'une photo.
pub async fn update_photo(
    db: &PgPool,
    id: Uuid,
    owner_id: Uuid,
    dto: UpdatePhotoDto,
) -> anyhow::Result<Option<Photo>> {
    let photo = sqlx::query_as::<_, Photo>(
        r#"UPDATE photos.photos
           SET description = COALESCE($1, description),
               is_starred  = COALESCE($2, is_starred),
               taken_at    = COALESCE($3, taken_at),
               updated_at  = NOW()
           WHERE id = $4 AND owner_id = $5
           RETURNING *"#,
    )
    .bind(dto.description.as_deref())
    .bind(dto.is_starred)
    .bind(dto.taken_at)
    .bind(id)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .context("update_photo")?;

    Ok(photo)
}

/// Déplace une photo vers la corbeille (soft delete).
pub async fn trash_photo(db: &PgPool, id: Uuid, owner_id: Uuid) -> anyhow::Result<bool> {
    let rows = sqlx::query(
        "UPDATE photos.photos SET is_trashed = TRUE, trashed_at = NOW(), updated_at = NOW()
         WHERE id = $1 AND owner_id = $2 AND is_trashed = FALSE",
    )
    .bind(id)
    .bind(owner_id)
    .execute(db)
    .await
    .context("trash_photo")?
    .rows_affected();

    Ok(rows > 0)
}

/// Restaure une photo de la corbeille.
pub async fn restore_photo(db: &PgPool, id: Uuid, owner_id: Uuid) -> anyhow::Result<bool> {
    let rows = sqlx::query(
        "UPDATE photos.photos SET is_trashed = FALSE, trashed_at = NULL, updated_at = NOW()
         WHERE id = $1 AND owner_id = $2 AND is_trashed = TRUE",
    )
    .bind(id)
    .bind(owner_id)
    .execute(db)
    .await
    .context("restore_photo")?
    .rows_affected();

    Ok(rows > 0)
}

/// Supprime définitivement une photo.
pub async fn delete_photo(
    db: &PgPool,
    storage: &dyn StorageBackend,
    id: Uuid,
    owner_id: Uuid,
) -> anyhow::Result<bool> {
    let photo = sqlx::query_as::<_, Photo>(
        "DELETE FROM photos.photos WHERE id = $1 AND owner_id = $2 RETURNING *",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .context("delete_photo")?;

    if let Some(p) = photo {
        let _ = storage.delete(&p.storage_path).await;
        let thumb_path = thumbnail_path(owner_id, id);
        let prev_path  = preview_path(owner_id, id);
        let _ = storage.delete(&thumb_path).await;
        let _ = storage.delete(&prev_path).await;
        Ok(true)
    } else {
        Ok(false)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn thumbnail_path(owner_id: Uuid, photo_id: Uuid) -> String {
    format!("photos/{owner_id}/thumbs/{photo_id}.jpg")
}

pub fn preview_path(owner_id: Uuid, photo_id: Uuid) -> String {
    format!("photos/{owner_id}/previews/{photo_id}.jpg")
}

fn is_image_mime(mime: &str) -> bool {
    matches!(
        mime,
        "image/jpeg" | "image/png" | "image/webp" | "image/gif"
        | "image/tiff" | "image/heic" | "image/heif" | "image/avif"
    )
}

fn extract_metadata(
    data: &Bytes,
) -> (Option<i32>, Option<i32>, Option<DateTime<Utc>>, Option<String>, Option<String>, Option<f64>, Option<f64>) {
    let mut width:        Option<i32> = None;
    let mut height:       Option<i32> = None;
    let mut taken_at:     Option<DateTime<Utc>> = None;
    let mut camera_make:  Option<String> = None;
    let mut camera_model: Option<String> = None;
    let mut gps_lat:      Option<f64> = None;
    let mut gps_lon:      Option<f64> = None;

    // Dimensions via image crate
    if let Ok(reader) = image::ImageReader::new(std::io::Cursor::new(data.as_ref()))
        .with_guessed_format()
    {
        if let Ok((w, h)) = reader.into_dimensions() {
            width  = Some(w as i32);
            height = Some(h as i32);
        }
    }

    // EXIF via kamadak-exif
    if let Ok(exif) = {
        let mut cur = std::io::Cursor::new(data.as_ref());
        exif::Reader::new().read_from_container(&mut cur)
    } {
        // Date de prise de vue
        if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
            if let exif::Value::Ascii(ref v) = field.value {
                if let Some(s) = v.first().and_then(|b| std::str::from_utf8(b).ok()) {
                    // Format EXIF : "2024:01:15 14:30:00"
                    let s = s.replace(':', "-").replacen('-', ":", 1).replacen('-', ":", 1);
                    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
                        taken_at = Some(dt.and_utc());
                    }
                }
            }
        }

        // Appareil photo
        if let Some(field) = exif.get_field(exif::Tag::Make, exif::In::PRIMARY) {
            camera_make = Some(field.display_value().to_string());
        }
        if let Some(field) = exif.get_field(exif::Tag::Model, exif::In::PRIMARY) {
            camera_model = Some(field.display_value().to_string());
        }

        // GPS
        if let (Some(lat_field), Some(lat_ref), Some(lon_field), Some(lon_ref)) = (
            exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY),
            exif.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY),
            exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY),
            exif.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY),
        ) {
            if let (exif::Value::Rational(lat_vals), exif::Value::Ascii(lat_ref_vals),
                    exif::Value::Rational(lon_vals), exif::Value::Ascii(lon_ref_vals)) =
                (&lat_field.value, &lat_ref.value, &lon_field.value, &lon_ref.value)
            {
                if lat_vals.len() >= 3 && lon_vals.len() >= 3 {
                    let lat = rational_to_deg(&lat_vals[0], &lat_vals[1], &lat_vals[2]);
                    let lon = rational_to_deg(&lon_vals[0], &lon_vals[1], &lon_vals[2]);
                    let lat_sign = if lat_ref_vals.first().and_then(|b| b.first()).copied() == Some(b'S') { -1.0 } else { 1.0 };
                    let lon_sign = if lon_ref_vals.first().and_then(|b| b.first()).copied() == Some(b'W') { -1.0 } else { 1.0 };
                    gps_lat = Some(lat * lat_sign);
                    gps_lon = Some(lon * lon_sign);
                }
            }
        }
    }

    (width, height, taken_at, camera_make, camera_model, gps_lat, gps_lon)
}

fn rational_to_deg(deg: &exif::Rational, min: &exif::Rational, sec: &exif::Rational) -> f64 {
    deg.to_f64() + min.to_f64() / 60.0 + sec.to_f64() / 3600.0
}

async fn generate_derivatives(
    storage: &dyn StorageBackend,
    owner_id: Uuid,
    photo_id: Uuid,
    data: &Bytes,
    thumbnail_size: u32,
    preview_size: u32,
) -> (bool, bool) {
    let has_thumbnail = generate_resized(
        storage,
        data,
        &thumbnail_path(owner_id, photo_id),
        thumbnail_size,
    ).await;

    let has_preview = generate_resized(
        storage,
        data,
        &preview_path(owner_id, photo_id),
        preview_size,
    ).await;

    (has_thumbnail, has_preview)
}

pub async fn generate_resized_pub(
    storage: &dyn StorageBackend,
    data: &Bytes,
    path: &str,
    size: u32,
) -> bool {
    generate_resized(storage, data, path, size).await
}

async fn generate_resized(
    storage: &dyn StorageBackend,
    data: &Bytes,
    path: &str,
    size: u32,
) -> bool {
    let data = data.clone();
    let path = path.to_string();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        let img = image::load_from_memory(&data)?;
        let resized = img.thumbnail(size, size);
        let mut buf = Vec::new();
        resized.write_to(
            &mut std::io::Cursor::new(&mut buf),
            image::ImageFormat::Jpeg,
        )?;
        Ok(buf)
    }).await;

    match result {
        Ok(Ok(bytes)) => {
            storage.put(&path, bytes.into()).await.is_ok()
        }
        _ => false,
    }
}
