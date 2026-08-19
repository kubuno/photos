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
    quality: u8,
    accept_undecodable: bool,
) -> anyhow::Result<Photo> {
    if data.len() as u64 > max_bytes {
        anyhow::bail!("FILE_TOO_LARGE");
    }

    let mime = mime_guess::from_path(original_name)
        .first_or_octet_stream()
        .to_string();

    // Seuls les formats image sont acceptés
    if !is_image_mime(&mime, accept_undecodable) {
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
    let (thumbnail_bytes, preview_bytes) = generate_derivatives(
        storage, owner_id, id, &data, thumbnail_size, preview_size, quality,
    ).await;
    // A non-zero weight is exactly the proof the derivative was written, so the
    // booleans are derived from it rather than tracked separately — the two can
    // never disagree.
    let has_thumbnail = thumbnail_bytes > 0;
    let has_preview   = preview_bytes > 0;
    let derived_bytes = (thumbnail_bytes + preview_bytes) as i64;

    let photo = sqlx::query_as::<_, Photo>(
        r#"INSERT INTO photos.photos
           (id, owner_id, filename, original_name, mime_type, size_bytes, width, height,
            storage_path, content_hash, taken_at, camera_make, camera_model,
            gps_lat, gps_lon, has_thumbnail, has_preview, derived_bytes)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
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
    .bind(derived_bytes)
    .fetch_one(db)
    .await
    .context("Insertion photo en DB")?;

    // An upload moves this account's declared total; the reporter coalesces the
    // mark and declares a few seconds later, so the upload never waits on the core.
    crate::services::usage::mark_dirty(owner_id);

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

    // Trashing does not free a byte, it moves it from `content` to `trash` — two
    // separate lines the core bills together. Both change, so the account is
    // re-declared.
    if rows > 0 {
        crate::services::usage::mark_dirty(owner_id);
    }

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

    if rows > 0 {
        crate::services::usage::mark_dirty(owner_id);
    }

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
        // A hard delete is the one operation that actually lowers the figure —
        // declaring it promptly is what stops a freed quota from looking used.
        crate::services::usage::mark_dirty(owner_id);
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

/// Formats this build can actually decode — the only ones that get dimensions,
/// a thumbnail and a preview. Kept in step with the `image` crate features
/// declared in `Cargo.toml`.
fn is_decodable_image_mime(mime: &str) -> bool {
    matches!(
        mime,
        "image/jpeg" | "image/png" | "image/webp" | "image/gif" | "image/tiff"
    )
}

/// Formats accepted at import. The undecodable ones (HEIC/HEIF/AVIF) are stored
/// verbatim but stay without dimensions and without derivatives, which is why an
/// instance may refuse them outright — see `accepted_formats` in `module.toml`.
fn is_image_mime(mime: &str, accept_undecodable: bool) -> bool {
    is_decodable_image_mime(mime)
        || (accept_undecodable && matches!(mime, "image/heic" | "image/heif" | "image/avif"))
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

/// Generates both derivatives and reports what each one weighs.
///
/// The sizes are returned rather than discarded because photos declares a
/// `thumbnails` category to the core: derivatives occupy real space in the
/// storage backend, and the only moment their weight is known for free is the
/// moment they are written. Measuring them later means one `size()` round-trip
/// per file against the backend — see `usage::backfill_derived_bytes`, which
/// exists solely to repair rows written before this was recorded.
async fn generate_derivatives(
    storage: &dyn StorageBackend,
    owner_id: Uuid,
    photo_id: Uuid,
    data: &Bytes,
    thumbnail_size: u32,
    preview_size: u32,
    quality: u8,
) -> (u64, u64) {
    let thumbnail_bytes = generate_resized(
        storage,
        data,
        &thumbnail_path(owner_id, photo_id),
        thumbnail_size,
        quality,
    ).await;

    let preview_bytes = generate_resized(
        storage,
        data,
        &preview_path(owner_id, photo_id),
        preview_size,
        quality,
    ).await;

    (thumbnail_bytes, preview_bytes)
}

/// Writes one derivative. Returns the bytes stored, or 0 when nothing was written.
pub async fn generate_resized_pub(
    storage: &dyn StorageBackend,
    data: &Bytes,
    path: &str,
    size: u32,
    quality: u8,
) -> u64 {
    generate_resized(storage, data, path, size, quality).await
}

/// Returns the number of bytes actually stored, `0` meaning "no derivative".
///
/// A best-effort path: a photo whose derivative cannot be produced is still a
/// valid photo, so failures are reported as zero rather than propagated.
async fn generate_resized(
    storage: &dyn StorageBackend,
    data: &Bytes,
    path: &str,
    size: u32,
    quality: u8,
) -> u64 {
    let data = data.clone();
    let path = path.to_string();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        let img = image::load_from_memory(&data)?;
        let resized = img.thumbnail(size, size);
        let mut buf = Vec::new();
        // Encode at the instance JPEG quality rather than the crate default (75).
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            std::io::Cursor::new(&mut buf),
            quality,
        );
        encoder.encode_image(&resized)?;
        Ok(buf)
    }).await;

    match result {
        Ok(Ok(bytes)) => {
            let len = bytes.len() as u64;
            if storage.put(&path, bytes.into()).await.is_ok() {
                len
            } else {
                0
            }
        }
        _ => 0,
    }
}
