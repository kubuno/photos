//! Trash maintenance: a background auto-purge of photos that have sat in the
//! trash longer than the instance retention window.

use std::sync::Arc;
use std::time::Duration;

use kubuno_storage::StorageBackend;
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::photo_service::{preview_path, thumbnail_path};
use crate::state::AppState;

/// Permanently removes photos trashed longer than `retention_days` (all users),
/// deleting the original, thumbnail and preview blobs and declaring the freed
/// space. Bounded per run so a large backlog drains gradually. Returns the count
/// purged.
async fn purge_old_photos(
    db: &PgPool,
    storage: &Arc<dyn StorageBackend>,
    retention_days: i32,
) -> usize {
    let stale: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT id, owner_id, storage_path FROM photos.photos
         WHERE is_trashed = TRUE AND trashed_at IS NOT NULL
           AND trashed_at < NOW() - make_interval(days => $1)
         LIMIT 500",
    )
    .bind(retention_days)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let mut purged = 0usize;
    for (id, owner, path) in stale {
        let _ = storage.delete(&path).await;
        let _ = storage.delete(&thumbnail_path(owner, id)).await;
        let _ = storage.delete(&preview_path(owner, id)).await;
        if sqlx::query("DELETE FROM photos.photos WHERE id = $1")
            .bind(id)
            .execute(db)
            .await
            .is_ok()
        {
            crate::services::usage::mark_dirty(owner);
            purged += 1;
        }
    }
    purged
}

/// Background worker: hourly, purges photos trashed beyond the retention window.
/// The first pass is deferred by one interval so a fresh deploy never purges
/// immediately on boot. A retention of `0` means "never" — the pass is skipped.
pub async fn run_trash_cleaner(state: AppState) {
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        let retention = state.instance().trash_auto_delete_days;
        if retention <= 0 {
            continue;
        }
        let n = purge_old_photos(&state.db, &state.storage, retention).await;
        if n > 0 {
            tracing::info!("Auto-purge corbeille photos : {n} photo(s) supprimée(s) définitivement");
        }
    }
}
