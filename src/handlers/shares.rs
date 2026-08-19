use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{PhotosError, Result},
    middleware::PhotosUser,
    models::{CreateShareDto, Share},
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
) -> Result<Json<Value>> {
    let shares = sqlx::query_as::<_, Share>(
        "SELECT * FROM photos.shares WHERE owner_id = $1 ORDER BY created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(PhotosError::Database)?;

    Ok(Json(json!({ "shares": shares })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Json(dto): Json<CreateShareDto>,
) -> Result<(StatusCode, Json<Value>)> {
    if dto.photo_id.is_none() && dto.album_id.is_none() {
        return Err(PhotosError::Validation("photo_id ou album_id requis".into()));
    }
    if dto.photo_id.is_some() && dto.album_id.is_some() {
        return Err(PhotosError::Validation("photo_id et album_id sont mutuellement exclusifs".into()));
    }

    // Instance policy: public links can be switched off entirely, and a maximum
    // lifetime can be imposed. Every photos share is a public token link, so the
    // switch gates them all.
    let inst = state.instance();
    if !inst.allow_public_sharing {
        return Err(PhotosError::Forbidden);
    }

    let mut token_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut token_bytes);
    let token = URL_SAFE_NO_PAD.encode(token_bytes);

    // Apply the expiry ceiling. With a ceiling set, a link created with no expiry
    // — or asking for a longer one — is clamped to the ceiling rather than left
    // permanent.
    let now = chrono::Utc::now();
    let requested = dto.expires_in_days.filter(|d| *d > 0);
    let expires_at = if inst.share_link_max_days > 0 {
        let days = requested
            .map(|d| d.min(inst.share_link_max_days))
            .unwrap_or(inst.share_link_max_days);
        Some(now + chrono::Duration::days(days))
    } else {
        requested.map(|d| now + chrono::Duration::days(d))
    };

    let share = sqlx::query_as::<_, Share>(
        r#"INSERT INTO photos.shares (owner_id, photo_id, album_id, token, expires_at)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING *"#,
    )
    .bind(user.id)
    .bind(dto.photo_id)
    .bind(dto.album_id)
    .bind(&token)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await
    .map_err(PhotosError::Database)?;

    Ok((StatusCode::CREATED, Json(json!({ "share": share }))))
}

pub async fn revoke(
    State(state): State<AppState>,
    Extension(user): Extension<PhotosUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let rows = sqlx::query(
        "DELETE FROM photos.shares WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .execute(&state.db)
    .await
    .map_err(PhotosError::Database)?
    .rows_affected();

    if rows == 0 {
        return Err(PhotosError::NotFound(format!("Share {id}")));
    }

    Ok(StatusCode::NO_CONTENT)
}
