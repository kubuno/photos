use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use kubuno_db::{params, DbPool, DbValue};
use uuid::Uuid;

use kubuno_storage::StorageBackend;

use crate::models::{ListPhotosQuery, Photo, UpdatePhotoDto};

/// Liste les photos d'un utilisateur.
pub async fn list_photos(
    db: &DbPool,
    owner_id: Uuid,
    q: ListPhotosQuery,
) -> Result<Vec<Photo>> {
    let limit  = q.limit.unwrap_or(100).min(500);
    let offset = q.offset.unwrap_or(0);
    let backend = db.backend();

    // `NULLS LAST` has no portable spelling (MySQL rejects the syntax), so the
    // null-last order is expressed as `(taken_at IS NULL)` — a 0/1 sort key on
    // every engine.
    let photos = if q.trashed == Some(true) {
        db.fetch_all_as::<Photo>(
            r#"SELECT * FROM photos.photos
               WHERE owner_id = $1 AND is_trashed = TRUE
               ORDER BY trashed_at DESC
               LIMIT $2 OFFSET $3"#,
            params![owner_id, limit, offset],
        )
        .await
        .context("list_photos trashed")?
    } else if q.starred == Some(true) {
        db.fetch_all_as::<Photo>(
            r#"SELECT * FROM photos.photos
               WHERE owner_id = $1 AND is_starred = TRUE AND is_trashed = FALSE
               ORDER BY (taken_at IS NULL), taken_at DESC, created_at DESC
               LIMIT $2 OFFSET $3"#,
            params![owner_id, limit, offset],
        )
        .await
        .context("list_photos starred")?
    } else if let Some(album_id) = q.album_id {
        db.fetch_all_as::<Photo>(
            r#"SELECT p.* FROM photos.photos p
               INNER JOIN photos.album_photos ap ON ap.photo_id = p.id
               WHERE ap.album_id = $1 AND p.owner_id = $2 AND p.is_trashed = FALSE
               ORDER BY ap.added_at DESC
               LIMIT $3 OFFSET $4"#,
            params![album_id, owner_id, limit, offset],
        )
        .await
        .context("list_photos by album")?
    } else {
        // Built dynamically: the source query reused `$3`/`$4`/`$5` across an
        // `IS NULL OR …` pair, which the runtime layer refuses (a placeholder is
        // positional and cannot be reused). Each optional filter now appends its
        // own clause and binds once, in order.
        let mut sql = String::from(
            "SELECT * FROM photos.photos WHERE owner_id = $1 AND is_trashed = FALSE",
        );
        let mut p: Vec<DbValue> = params![owner_id];
        let mut n = 2usize;

        if let Some(from) = q.from {
            sql.push_str(&format!(" AND taken_at >= ${n}"));
            p.push(DbValue::from(from));
            n += 1;
        }
        if let Some(to) = q.to {
            sql.push_str(&format!(" AND taken_at <= ${n}"));
            p.push(DbValue::from(to));
            n += 1;
        }
        if let Some(search) = q.search.as_deref().filter(|s| !s.is_empty()) {
            // Bind the wildcards rather than concatenating with `||`, which is
            // logical OR (not string concat) on MySQL. The same pattern is bound
            // twice because a placeholder cannot be reused.
            let like = format!("%{search}%");
            let name_cond = backend.ilike("original_name", n);
            let desc_cond = backend.ilike("description", n + 1);
            sql.push_str(&format!(" AND ({name_cond} OR {desc_cond})"));
            p.push(DbValue::from(like.clone()));
            p.push(DbValue::from(like));
            n += 2;
        }

        sql.push_str(&format!(
            " ORDER BY (taken_at IS NULL), taken_at DESC, created_at DESC LIMIT ${} OFFSET ${}",
            n,
            n + 1,
        ));
        p.push(DbValue::from(limit));
        p.push(DbValue::from(offset));

        db.fetch_all_as::<Photo>(&sql, p).await.context("list_photos")?
    };

    Ok(photos)
}

/// Récupère une photo par ID (vérifie ownership).
pub async fn get_photo(db: &DbPool, id: Uuid, owner_id: Uuid) -> Result<Option<Photo>> {
    let photo = db
        .fetch_optional_as::<Photo>(
            "SELECT * FROM photos.photos WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .context("get_photo")?;

    Ok(photo)
}

/// Upload et enregistre une photo.
/// The knobs an administrator turns for uploads. They travel together —
/// they all come from the same instance settings and are all read on the same
/// path — so they are passed as one value rather than as five positional
/// arguments a caller could silently transpose.
pub struct UploadLimits {
    pub max_bytes: u64,
    pub thumbnail_size: u32,
    pub preview_size: u32,
    pub quality: u8,
    /// Store a file the decoder cannot read rather than refusing it.
    pub accept_undecodable: bool,
}

pub async fn upload_photo(
    db: &DbPool,
    storage: &dyn StorageBackend,
    owner_id: Uuid,
    original_name: &str,
    data: Bytes,
    limits: UploadLimits,
) -> anyhow::Result<Photo> {
    let UploadLimits { max_bytes, thumbnail_size, preview_size, quality, accept_undecodable } = limits;
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
    let PhotoMetadata { width, height, taken_at, camera_make, camera_model, gps_lat, gps_lon } =
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

    // The key is generated here and bound explicitly (see `id` above): MySQL and
    // SQLite have no `RETURNING`, so a database-invented key could not be read
    // back. With the key known before the write, the insert is a plain statement
    // and the row is read back by primary key — identical on the three engines.
    // `metadata` is bound explicitly (an empty JSON object) rather than left to a
    // column DEFAULT: a non-NULL JSON default is spelled inconsistently across
    // the three engines, and the column decodes into a non-optional
    // `serde_json::Value`.
    db.execute(
        r#"INSERT INTO photos.photos
           (id, owner_id, filename, original_name, mime_type, size_bytes, width, height,
            storage_path, content_hash, taken_at, camera_make, camera_model,
            gps_lat, gps_lon, has_thumbnail, has_preview, derived_bytes, metadata)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)"#,
        params![
            id,
            owner_id,
            &sanitized,
            original_name,
            &mime,
            data.len() as i64,
            width,
            height,
            &storage_path,
            &hash,
            taken_at,
            camera_make.as_deref(),
            camera_model.as_deref(),
            gps_lat,
            gps_lon,
            has_thumbnail,
            has_preview,
            derived_bytes,
            serde_json::json!({}),
        ],
    )
    .await
    .context("Insertion photo en DB")?;

    let photo = db
        .fetch_one_as::<Photo>(
            "SELECT * FROM photos.photos WHERE id = $1",
            params![id],
        )
        .await
        .context("Relecture de la photo insérée")?;

    // An upload moves this account's declared total; the reporter coalesces the
    // mark and declares a few seconds later, so the upload never waits on the core.
    crate::services::usage::mark_dirty(owner_id);

    Ok(photo)
}

/// Met à jour les métadonnées d'une photo.
pub async fn update_photo(
    db: &DbPool,
    id: Uuid,
    owner_id: Uuid,
    dto: UpdatePhotoDto,
) -> anyhow::Result<Option<Photo>> {
    // No `RETURNING`: the update is applied, then the row is re-read by its
    // primary key (and owner, so a wrong owner still yields `None`). The `WHERE`
    // is on immutable columns, so this is not the guarded-update case the runtime
    // layer cannot emulate. `updated_at` is bound from Rust rather than `NOW()`.
    db.execute(
        r#"UPDATE photos.photos
           SET description = COALESCE($1, description),
               is_starred  = COALESCE($2, is_starred),
               taken_at    = COALESCE($3, taken_at),
               updated_at  = $4
           WHERE id = $5 AND owner_id = $6"#,
        params![
            dto.description.as_deref(),
            dto.is_starred,
            dto.taken_at,
            Utc::now(),
            id,
            owner_id,
        ],
    )
    .await
    .context("update_photo")?;

    let photo = db
        .fetch_optional_as::<Photo>(
            "SELECT * FROM photos.photos WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .context("update_photo reselect")?;

    Ok(photo)
}

/// Déplace une photo vers la corbeille (soft delete).
pub async fn trash_photo(db: &DbPool, id: Uuid, owner_id: Uuid) -> anyhow::Result<bool> {
    // `NOW()` is bound from Rust (spelled differently per engine, and SQLite has
    // no time zone). The same instant is bound twice: a placeholder cannot be
    // reused.
    let now = Utc::now();
    let rows = db
        .execute(
            "UPDATE photos.photos SET is_trashed = TRUE, trashed_at = $1, updated_at = $2
             WHERE id = $3 AND owner_id = $4 AND is_trashed = FALSE",
            params![now, now, id, owner_id],
        )
        .await
        .context("trash_photo")?;

    // Trashing does not free a byte, it moves it from `content` to `trash` — two
    // separate lines the core bills together. Both change, so the account is
    // re-declared.
    if rows > 0 {
        crate::services::usage::mark_dirty(owner_id);
    }

    Ok(rows > 0)
}

/// Restaure une photo de la corbeille.
pub async fn restore_photo(db: &DbPool, id: Uuid, owner_id: Uuid) -> anyhow::Result<bool> {
    let rows = db
        .execute(
            "UPDATE photos.photos SET is_trashed = FALSE, trashed_at = NULL, updated_at = $1
             WHERE id = $2 AND owner_id = $3 AND is_trashed = TRUE",
            params![Utc::now(), id, owner_id],
        )
        .await
        .context("restore_photo")?;

    if rows > 0 {
        crate::services::usage::mark_dirty(owner_id);
    }

    Ok(rows > 0)
}

/// Supprime définitivement une photo.
pub async fn delete_photo(
    db: &DbPool,
    storage: &dyn StorageBackend,
    id: Uuid,
    owner_id: Uuid,
) -> anyhow::Result<bool> {
    // A delete that must return the row is a read-before-write on an engine
    // without `RETURNING`: the row is looked up first (its `storage_path` is what
    // the caller needs), then removed.
    let photo = db
        .fetch_optional_as::<Photo>(
            "SELECT * FROM photos.photos WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .context("delete_photo lookup")?;

    if let Some(p) = photo {
        db.execute(
            "DELETE FROM photos.photos WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .context("delete_photo")?;

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

/// What a photo file says about itself: its dimensions, and whatever EXIF
/// carries. Every field is optional — a file may have no EXIF at all, or an
/// unreadable one, and that is not an error.
#[derive(Debug, Default)]
struct PhotoMetadata {
    width:        Option<i32>,
    height:       Option<i32>,
    taken_at:     Option<DateTime<Utc>>,
    camera_make:  Option<String>,
    camera_model: Option<String>,
    gps_lat:      Option<f64>,
    gps_lon:      Option<f64>,
}

fn extract_metadata(data: &Bytes) -> PhotoMetadata {
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

    PhotoMetadata { width, height, taken_at, camera_make, camera_model, gps_lat, gps_lon }
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
