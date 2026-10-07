use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Share {
    pub id:         Uuid,
    pub owner_id:   Uuid,
    pub photo_id:   Option<Uuid>,
    pub album_id:   Option<Uuid>,
    pub token:      String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateShareDto {
    pub photo_id:   Option<Uuid>,
    pub album_id:   Option<Uuid>,
    pub expires_in_days: Option<i64>,
}
