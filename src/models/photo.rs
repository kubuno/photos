use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Photo {
    pub id:            Uuid,
    pub owner_id:      Uuid,
    pub filename:      String,
    pub original_name: String,
    pub mime_type:     String,
    pub size_bytes:    i64,
    pub width:         Option<i32>,
    pub height:        Option<i32>,
    pub storage_path:  String,
    pub content_hash:  Option<String>,
    pub taken_at:      Option<DateTime<Utc>>,
    pub camera_make:   Option<String>,
    pub camera_model:  Option<String>,
    pub gps_lat:       Option<f64>,
    pub gps_lon:       Option<f64>,
    pub has_thumbnail: bool,
    pub has_preview:   bool,
    pub is_starred:    bool,
    pub is_trashed:    bool,
    pub trashed_at:    Option<DateTime<Utc>>,
    pub description:   Option<String>,
    pub metadata:      serde_json::Value,
    pub created_at:    DateTime<Utc>,
    pub updated_at:    DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListPhotosQuery {
    pub album_id:  Option<Uuid>,
    pub starred:   Option<bool>,
    pub trashed:   Option<bool>,
    pub from:      Option<DateTime<Utc>>,
    pub to:        Option<DateTime<Utc>>,
    pub search:    Option<String>,
    pub limit:     Option<i64>,
    pub offset:    Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePhotoDto {
    pub description: Option<String>,
    pub is_starred:  Option<bool>,
    pub taken_at:    Option<DateTime<Utc>>,
}
