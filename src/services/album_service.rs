use anyhow::{Context, Result};
use chrono::Utc;
use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::models::{AddPhotosToAlbumDto, Album, CreateAlbumDto, UpdateAlbumDto};

/// The base `SELECT` for an album with its live photo count.
///
/// `COUNT(*)` does not return the same SQL type on the three engines, so it is
/// wrapped by `count_bigint` (a cast to a signed 64-bit integer) — otherwise
/// MySQL's `BIGINT UNSIGNED` would not decode into `Album::photo_count` (`i64`).
fn album_select(backend: kubuno_db::Backend) -> String {
    format!(
        "SELECT a.*, \
           COALESCE((SELECT {count} FROM photos.album_photos ap WHERE ap.album_id = a.id), 0) AS photo_count \
         FROM photos.albums a",
        count = backend.count_bigint("*"),
    )
}

pub async fn list_albums(db: &DbPool, owner_id: Uuid) -> Result<Vec<Album>> {
    let sql = format!(
        "{base} WHERE a.owner_id = $1 ORDER BY a.updated_at DESC",
        base = album_select(db.backend()),
    );
    let albums = db
        .fetch_all_as::<Album>(&sql, params![owner_id])
        .await
        .context("list_albums")?;

    Ok(albums)
}

pub async fn get_album(db: &DbPool, id: Uuid, owner_id: Uuid) -> Result<Option<Album>> {
    let sql = format!(
        "{base} WHERE a.id = $1 AND a.owner_id = $2",
        base = album_select(db.backend()),
    );
    let album = db
        .fetch_optional_as::<Album>(&sql, params![id, owner_id])
        .await
        .context("get_album")?;

    Ok(album)
}

pub async fn create_album(db: &DbPool, owner_id: Uuid, dto: CreateAlbumDto) -> Result<Album> {
    // The key is generated here and bound (no `RETURNING` on MySQL/SQLite). The
    // row is then read back with the same projection as `get_album`, so a fresh
    // album reports `photo_count = 0` without a special-cased literal.
    let id = kubuno_db::new_id();
    db.execute(
        "INSERT INTO photos.albums (id, owner_id, name, description) VALUES ($1, $2, $3, $4)",
        params![id, owner_id, &dto.name, dto.description.as_deref()],
    )
    .await
    .context("create_album")?;

    let sql = format!(
        "{base} WHERE a.id = $1",
        base = album_select(db.backend()),
    );
    let album = db
        .fetch_one_as::<Album>(&sql, params![id])
        .await
        .context("create_album reselect")?;

    Ok(album)
}

pub async fn update_album(
    db: &DbPool,
    id: Uuid,
    owner_id: Uuid,
    dto: UpdateAlbumDto,
) -> Result<Option<Album>> {
    db.execute(
        r#"UPDATE photos.albums
           SET name           = COALESCE($1, name),
               description    = COALESCE($2, description),
               cover_photo_id = COALESCE($3, cover_photo_id),
               updated_at     = $4
           WHERE id = $5 AND owner_id = $6"#,
        params![
            dto.name.as_deref(),
            dto.description.as_deref(),
            dto.cover_photo_id,
            Utc::now(),
            id,
            owner_id,
        ],
    )
    .await
    .context("update_album")?;

    let sql = format!(
        "{base} WHERE a.id = $1 AND a.owner_id = $2",
        base = album_select(db.backend()),
    );
    let album = db
        .fetch_optional_as::<Album>(&sql, params![id, owner_id])
        .await
        .context("update_album reselect")?;

    Ok(album)
}

pub async fn delete_album(db: &DbPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let rows = db
        .execute(
            "DELETE FROM photos.albums WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .context("delete_album")?;

    Ok(rows > 0)
}

pub async fn add_photos(
    db: &DbPool,
    album_id: Uuid,
    owner_id: Uuid,
    dto: AddPhotosToAlbumDto,
) -> Result<usize> {
    let backend = db.backend();

    // Vérifier que l'album appartient bien à l'utilisateur. `EXISTS` does not
    // decode to a uniform Rust type across engines (a boolean on PostgreSQL, an
    // integer elsewhere), so ownership is checked with a counted `i64`.
    let owned: i64 = db
        .fetch_scalar(
            &format!(
                "SELECT {count} FROM photos.albums WHERE id = $1 AND owner_id = $2",
                count = backend.count_bigint("*"),
            ),
            params![album_id, owner_id],
        )
        .await
        .context("add_photos: check album ownership")?;

    if owned == 0 {
        return Ok(0);
    }

    // `ON CONFLICT DO NOTHING` is spelled in two halves: a prefix (`INSERT
    // IGNORE` on MySQL) and a trailing clause (`ON CONFLICT (...) DO NOTHING` on
    // PostgreSQL/SQLite).
    let insert_sql = format!(
        "INSERT {ignore}INTO photos.album_photos (album_id, photo_id) VALUES ($1, $2){on_conflict}",
        ignore = backend.insert_ignore_prefix(),
        on_conflict = backend.on_conflict_do_nothing(&["album_id", "photo_id"]),
    );

    let mut count = 0usize;
    for photo_id in &dto.photo_ids {
        let rows = db
            .execute(&insert_sql, params![album_id, photo_id])
            .await
            .context("add_photos insert")?;
        count += rows as usize;
    }

    // Mettre à jour updated_at de l'album
    db.execute(
        "UPDATE photos.albums SET updated_at = $1 WHERE id = $2",
        params![Utc::now(), album_id],
    )
    .await
    .context("add_photos: update album")?;

    Ok(count)
}

pub async fn remove_photo(
    db: &DbPool,
    album_id: Uuid,
    photo_id: Uuid,
    owner_id: Uuid,
) -> Result<bool> {
    // `album_id` is bound twice (once for the row, once for the ownership
    // subquery): a placeholder is positional and cannot be reused.
    let rows = db
        .execute(
            r#"DELETE FROM photos.album_photos
               WHERE album_id = $1 AND photo_id = $2
                 AND (SELECT owner_id FROM photos.albums WHERE id = $3) = $4"#,
            params![album_id, photo_id, album_id, owner_id],
        )
        .await
        .context("remove_photo from album")?;

    Ok(rows > 0)
}
