//! Unauthenticated side of a share link.
//!
//! ## Two permissions, not one
//!
//! Until now a link had a single door: `/download`, which handed over the
//! ORIGINAL file and doubled as the way to display it. That made "may look at
//! it" and "may keep a copy of it" the same permission, so an instance could
//! only ever allow both or neither — the switch an administrator actually wants
//! ("recipients may view, not download") had nowhere to attach.
//!
//! So the doors are separated:
//!
//! * `/preview` — the downscaled derivative. This is the VIEWING path, always
//!   open for a valid link. It falls back to the original when a preview was
//!   never generated (an undecodable format), because refusing to show a shared
//!   photo would be a worse answer than showing it at full size.
//! * `/download` — the original, as an attachment, and the only one
//!   `share_allow_download` closes.
//!
//! ## What a link tells about the picture
//!
//! Capture metadata is the other half of the policy. EXIF carries the camera,
//! the date, and — the one that matters — the GPS coordinates of where the
//! photo was taken. A link handed to the outside world discloses none of it
//! unless the administrator turns `share_expose_metadata` on.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde_json::{json, Value};

use crate::{
    errors::{PhotosError, Result},
    models::{Photo, Share},
    services::photo_service,
    state::AppState,
};

async fn get_valid_share(state: &AppState, token: &str) -> Result<Share> {
    let share = sqlx::query_as::<_, Share>(
        "SELECT * FROM photos.shares WHERE token = $1",
    )
    .bind(token)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Lecture d'un partage public échouée");
        PhotosError::Database(e)
    })?
    .ok_or_else(|| PhotosError::NotFound("Partage introuvable".into()))?;

    if let Some(exp) = share.expires_at {
        if exp < chrono::Utc::now() {
            return Err(PhotosError::Forbidden);
        }
    }

    Ok(share)
}

/// The shared photo, or a validation error for an album share (never served).
async fn shared_photo(state: &AppState, share: &Share) -> Result<Photo> {
    let photo_id = share.photo_id
        .ok_or_else(|| PhotosError::Validation("Ce partage concerne un album, pas une photo".into()))?;

    photo_service::get_photo(&state.db, photo_id, share.owner_id)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound("Photo introuvable".into()))
}

pub async fn info(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<Value>> {
    let share = get_valid_share(&state, &token).await?;
    let inst  = state.instance();

    // The recipient is told what they may do BEFORE trying: a download button
    // that answers 403 is worse than one that was never shown.
    let mut body = json!({
        "share":        share,
        "can_download": inst.share_allow_download,
    });

    if let Ok(photo) = shared_photo(&state, &share).await {
        let mut projection = json!({
            "id":            photo.id,
            "original_name": photo.original_name,
            "mime_type":     photo.mime_type,
            "width":         photo.width,
            "height":        photo.height,
        });

        // Capture metadata — camera, date, and above all the coordinates — is
        // added only when the instance allows a public link to disclose it.
        if inst.share_expose_metadata {
            if let Some(obj) = projection.as_object_mut() {
                obj.insert("taken_at".into(),     json!(photo.taken_at));
                obj.insert("camera_make".into(),  json!(photo.camera_make));
                obj.insert("camera_model".into(), json!(photo.camera_model));
                obj.insert("gps_lat".into(),      json!(photo.gps_lat));
                obj.insert("gps_lon".into(),      json!(photo.gps_lon));
            }
        }

        if let Some(obj) = body.as_object_mut() {
            obj.insert("photo".into(), projection);
        }
    }

    Ok(Json(body))
}

/// The viewing path: the downscaled preview, inline. Never gated by
/// `share_allow_download` — a link that shows nothing is not a share.
pub async fn preview(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response> {
    let share = get_valid_share(&state, &token).await?;
    let photo = shared_photo(&state, &share).await?;

    // No preview means the format was never decodable; showing the original is
    // the only remaining way to honour the link.
    let (path, mime) = if photo.has_preview {
        (photo_service::preview_path(photo.owner_id, photo.id), "image/jpeg".to_string())
    } else {
        (photo.storage_path.clone(), photo.mime_type.clone())
    };

    let data = state.storage.get(&path).await.map_err(PhotosError::Storage)?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_DISPOSITION, "inline")
        .body(Body::from(data))
        .map_err(|e| PhotosError::Internal(e.into()))
}

/// The taking-away path: the original file. This is the one an instance closes.
pub async fn download(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response> {
    let share = get_valid_share(&state, &token).await?;

    // Instance policy: a valid link may be view-only. Unlike the creation-time
    // switch (`allow_public_sharing`), this one applies to links ALREADY handed
    // out — it is the answer to "we shared too much last month".
    if !state.instance().share_allow_download {
        return Err(PhotosError::Forbidden);
    }

    let photo = shared_photo(&state, &share).await?;

    let data = state.storage.get(&photo.storage_path)
        .await
        .map_err(PhotosError::Storage)?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &photo.mime_type)
        .header(
            header::CONTENT_DISPOSITION,
            // Kept `inline`: links handed out before `/preview` existed point
            // here, and turning them into forced downloads would change what
            // recipients see for shares nobody re-issued.
            format!("inline; filename=\"{}\"", photo.original_name),
        )
        .body(Body::from(data))
        .map_err(|e| PhotosError::Internal(e.into()))
}
