use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Album {
    pub id:             Uuid,
    pub owner_id:       Uuid,
    pub name:           String,
    pub description:    Option<String>,
    pub cover_photo_id: Option<Uuid>,
    pub photo_count:    i64,
    pub is_shared:      bool,
    pub share_token:    Option<String>,
    pub created_at:     DateTime<Utc>,
    pub updated_at:     DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAlbumDto {
    pub name:        String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAlbumDto {
    pub name:           Option<String>,
    pub description:    Option<String>,
    pub cover_photo_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct AddPhotosToAlbumDto {
    pub photo_ids: Vec<Uuid>,
}
