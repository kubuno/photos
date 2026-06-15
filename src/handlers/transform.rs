use axum::{extract::{Path, State}, Extension, Json};
use bytes::Bytes;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Cursor;
use uuid::Uuid;

use crate::{
    errors::{PhotosError, Result},
    middleware::PhotosUser,
    services::photo_service::{self, thumbnail_path, preview_path, generate_resized_pub},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct TransformDto {
    /// Rotation clockwise en degrés : 90, 180, 270
    pub rotate: Option<i32>,
    pub flip_h: Option<bool>,
    pub flip_v: Option<bool>,
}

pub async fn transform(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(photo_id): Path<Uuid>,
    Json(dto): Json<TransformDto>,
) -> Result<Json<Value>> {
    let photo = photo_service::get_photo(&state.db, photo_id, user.id)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound(format!("Photo {photo_id}")))?;

    let data   = state.storage.get(&photo.storage_path).await?;
    let rotate = dto.rotate.unwrap_or(0);
    let flip_h = dto.flip_h.unwrap_or(false);
    let flip_v = dto.flip_v.unwrap_or(false);
    let mime   = photo.mime_type.clone();

    let encoded = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        let mut img = image::load_from_memory(&data)?;

        let deg = ((rotate % 360) + 360) % 360;
        img = match deg {
            90  => img.rotate90(),
            180 => img.rotate180(),
            270 => img.rotate270(),
            _   => img,
        };
        if flip_h { img = img.fliph(); }
        if flip_v { img = img.flipv(); }

        let fmt = image::ImageFormat::from_mime_type(&mime)
            .unwrap_or(image::ImageFormat::Jpeg);
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), fmt)?;
        Ok(buf)
    })
    .await
    .map_err(|e| PhotosError::Internal(anyhow::anyhow!("{e}")))?
    .map_err(PhotosError::Internal)?;

    let new_size  = encoded.len() as i64;
    let new_bytes = Bytes::from(encoded);

    state.storage.put(&photo.storage_path, new_bytes.clone()).await?;

    // Regénérer thumbnail + preview (best-effort)
    {
        let db      = state.db.clone();
        let storage = state.storage.clone();
        let owner   = user.id;
        let pid     = photo_id;
        let ts      = state.settings.photos.thumbnail_size;
        let ps      = state.settings.photos.preview_size;
        let nb      = new_bytes;
        tokio::spawn(async move {
            let _ = sqlx::query(
                "UPDATE photos.photos SET size_bytes = $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(new_size)
            .bind(pid)
            .execute(&db)
            .await;

            generate_resized_pub(storage.as_ref(), &nb, &thumbnail_path(owner, pid), ts).await;
            generate_resized_pub(storage.as_ref(), &nb, &preview_path(owner, pid), ps).await;
        });
    }

    let updated = photo_service::get_photo(&state.db, photo_id, user.id)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound(format!("Photo {photo_id}")))?;

    Ok(Json(json!({ "photo": updated })))
}
