use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};

pub async fn publish_event(
    client: &Client,
    core_url: &str,
    internal_secret: &str,
    event: Value,
) -> Result<()> {
    let url = format!("{core_url}/internal/events/publish");
    client
        .post(&url)
        .header("X-Internal-Secret", internal_secret)
        .json(&event)
        .send()
        .await?;
    Ok(())
}

pub fn photo_imported_event(
    photo_id: uuid::Uuid,
    user_id: uuid::Uuid,
    mime_type: &str,
    size_bytes: i64,
) -> Value {
    json!({
        "type": "PhotoImported",
        "payload": {
            "photo_id":   photo_id,
            "user_id":    user_id,
            "mime_type":  mime_type,
            "size_bytes": size_bytes,
            "module_id":  "photos"
        }
    })
}

pub fn photo_deleted_event(photo_id: uuid::Uuid, user_id: uuid::Uuid) -> Value {
    json!({
        "type": "FileDeleted",
        "payload": {
            "file_id":   photo_id,
            "user_id":   user_id,
            "module_id": "photos"
        }
    })
}
