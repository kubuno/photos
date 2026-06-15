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
    models::Share,
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
    .map_err(PhotosError::Database)?
    .ok_or_else(|| PhotosError::NotFound("Partage introuvable".into()))?;

    if let Some(exp) = share.expires_at {
        if exp < chrono::Utc::now() {
            return Err(PhotosError::Forbidden);
        }
    }

    Ok(share)
}

pub async fn info(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<Value>> {
    let share = get_valid_share(&state, &token).await?;
    Ok(Json(json!({ "share": share })))
}

pub async fn download(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response> {
    let share = get_valid_share(&state, &token).await?;

    let photo_id = share.photo_id
        .ok_or_else(|| PhotosError::Validation("Ce partage concerne un album, pas une photo".into()))?;

    let photo = photo_service::get_photo(&state.db, photo_id, share.owner_id)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound("Photo introuvable".into()))?;

    let data = state.storage.get(&photo.storage_path)
        .await
        .map_err(PhotosError::Storage)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &photo.mime_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", photo.original_name),
        )
        .body(Body::from(data))
        .map_err(|e| PhotosError::Internal(e.into()))?;

    Ok(response)
}
