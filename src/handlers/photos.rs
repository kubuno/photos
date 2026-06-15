use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{PhotosError, Result},
    events,
    middleware::PhotosUser,
    models::{ListPhotosQuery, UpdatePhotoDto},
    services::photo_service,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Query(q): Query<ListPhotosQuery>,
) -> Result<Json<Value>> {
    let photos = photo_service::list_photos(&state.db, user.id, q)
        .await
        .map_err(|e| PhotosError::Internal(e))?;
    Ok(Json(json!({ "photos": photos })))
}

pub async fn upload(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    mut multipart: Multipart,
) -> Result<Json<Value>> {
    let max          = state.settings.photos.max_upload_bytes;
    let thumb_size   = state.settings.photos.thumbnail_size;
    let preview_size = state.settings.photos.preview_size;
    let mut filename: Option<String> = None;
    let mut data: Option<bytes::Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| PhotosError::Validation(e.to_string()))?
    {
        if matches!(field.name(), Some("photo") | Some("file")) {
            let name = field.file_name().unwrap_or("photo.jpg").to_string();
            let bytes = field.bytes().await
                .map_err(|e| PhotosError::Validation(e.to_string()))?;
            filename = Some(name);
            data = Some(bytes);
        }
    }

    let name  = filename.ok_or_else(|| PhotosError::Validation("Champ 'photo' manquant".into()))?;
    let bytes = data.ok_or_else(|| PhotosError::Validation("Données manquantes".into()))?;

    let photo = photo_service::upload_photo(
        &state.db,
        state.storage.as_ref(),
        user.id,
        &name,
        bytes,
        max,
        thumb_size,
        preview_size,
    )
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg == "FILE_TOO_LARGE" { PhotosError::FileTooLarge }
        else if msg == "UNSUPPORTED_FORMAT" { PhotosError::UnsupportedFormat }
        else { PhotosError::Internal(e) }
    })?;

    // Publier l'event (best-effort)
    {
        let ev  = events::photo_imported_event(photo.id, user.id, &photo.mime_type, photo.size_bytes);
        let http    = state.http.clone();
        let core    = state.settings.core.url.clone();
        let secret  = state.settings.core.internal_secret.clone();
        tokio::spawn(async move {
            let _ = events::publish_event(&http, &core, &secret, ev).await;
        });
    }

    Ok(Json(json!({ "photo": photo })))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let photo = photo_service::get_photo(&state.db, id, user.id)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound(format!("Photo {id}")))?;

    Ok(Json(json!({ "photo": photo })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdatePhotoDto>,
) -> Result<Json<Value>> {
    let photo = photo_service::update_photo(&state.db, id, user.id, dto)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound(format!("Photo {id}")))?;

    Ok(Json(json!({ "photo": photo })))
}

pub async fn download(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    let photo = photo_service::get_photo(&state.db, id, user.id)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound(format!("Photo {id}")))?;

    let data = state.storage.get(&photo.storage_path)
        .await
        .map_err(PhotosError::Storage)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &photo.mime_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", photo.original_name),
        )
        .body(Body::from(data))
        .map_err(|e| PhotosError::Internal(e.into()))?;

    Ok(response)
}

pub async fn thumbnail(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    let photo = photo_service::get_photo(&state.db, id, user.id)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound(format!("Photo {id}")))?;

    let thumb_path = photo_service::thumbnail_path(user.id, photo.id);
    let data = state.storage.get(&thumb_path)
        .await
        .map_err(PhotosError::Storage)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .body(Body::from(data))
        .map_err(|e| PhotosError::Internal(e.into()))?;

    Ok(response)
}

pub async fn preview(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    let photo = photo_service::get_photo(&state.db, id, user.id)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound(format!("Photo {id}")))?;

    let prev_path = photo_service::preview_path(user.id, photo.id);
    let data = state.storage.get(&prev_path).await;

    // Fallback sur l'image originale si le preview n'existe pas
    let (content_type, body_data) = match data {
        Ok(d)  => ("image/jpeg".to_string(), d),
        Err(_) => {
            let orig = state.storage.get(&photo.storage_path).await
                .map_err(PhotosError::Storage)?;
            (photo.mime_type.clone(), orig)
        }
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(body_data))
        .map_err(|e| PhotosError::Internal(e.into()))?;

    Ok(response)
}

pub async fn trash(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let found = photo_service::trash_photo(&state.db, id, user.id)
        .await
        .map_err(PhotosError::Internal)?;

    if !found {
        return Err(PhotosError::NotFound(format!("Photo {id}")));
    }

    Ok(Json(json!({ "message": "Photo déplacée vers la corbeille" })))
}

pub async fn restore(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let found = photo_service::restore_photo(&state.db, id, user.id)
        .await
        .map_err(PhotosError::Internal)?;

    if !found {
        return Err(PhotosError::NotFound(format!("Photo {id}")));
    }

    Ok(Json(json!({ "message": "Photo restaurée" })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let found = photo_service::delete_photo(&state.db, state.storage.as_ref(), id, user.id)
        .await
        .map_err(PhotosError::Internal)?;

    if !found {
        return Err(PhotosError::NotFound(format!("Photo {id}")));
    }

    // Publier event suppression (best-effort)
    {
        let ev     = events::photo_deleted_event(id, user.id);
        let http   = state.http.clone();
        let core   = state.settings.core.url.clone();
        let secret = state.settings.core.internal_secret.clone();
        tokio::spawn(async move {
            let _ = events::publish_event(&http, &core, &secret, ev).await;
        });
    }

    Ok(StatusCode::NO_CONTENT)
}
