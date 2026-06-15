use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{PhotosError, Result},
    middleware::PhotosUser,
    models::{AddPhotosToAlbumDto, CreateAlbumDto, ListPhotosQuery, UpdateAlbumDto},
    services::{album_service, photo_service},
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
) -> Result<Json<Value>> {
    let albums = album_service::list_albums(&state.db, user.id)
        .await
        .map_err(PhotosError::Internal)?;
    Ok(Json(json!({ "albums": albums })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Json(dto): Json<CreateAlbumDto>,
) -> Result<(StatusCode, Json<Value>)> {
    if dto.name.trim().is_empty() {
        return Err(PhotosError::Validation("Le nom de l'album est requis".into()));
    }
    let album = album_service::create_album(&state.db, user.id, dto)
        .await
        .map_err(PhotosError::Internal)?;
    Ok((StatusCode::CREATED, Json(json!({ "album": album }))))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let album = album_service::get_album(&state.db, id, user.id)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound(format!("Album {id}")))?;
    Ok(Json(json!({ "album": album })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateAlbumDto>,
) -> Result<Json<Value>> {
    let album = album_service::update_album(&state.db, id, user.id, dto)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound(format!("Album {id}")))?;
    Ok(Json(json!({ "album": album })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let found = album_service::delete_album(&state.db, id, user.id)
        .await
        .map_err(PhotosError::Internal)?;
    if !found {
        return Err(PhotosError::NotFound(format!("Album {id}")));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_photos(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
    axum::extract::Query(mut q): axum::extract::Query<ListPhotosQuery>,
) -> Result<Json<Value>> {
    // Vérifier que l'album appartient à l'utilisateur
    album_service::get_album(&state.db, id, user.id)
        .await
        .map_err(PhotosError::Internal)?
        .ok_or_else(|| PhotosError::NotFound(format!("Album {id}")))?;

    q.album_id = Some(id);
    let photos = photo_service::list_photos(&state.db, user.id, q)
        .await
        .map_err(PhotosError::Internal)?;
    Ok(Json(json!({ "photos": photos })))
}

pub async fn add_photos(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<AddPhotosToAlbumDto>,
) -> Result<Json<Value>> {
    let count = album_service::add_photos(&state.db, id, user.id, dto)
        .await
        .map_err(PhotosError::Internal)?;
    Ok(Json(json!({ "added": count })))
}

pub async fn remove_photo(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path((album_id, photo_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    album_service::remove_photo(&state.db, album_id, photo_id, user.id)
        .await
        .map_err(PhotosError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}
