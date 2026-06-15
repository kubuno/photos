use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{AddPhotosToAlbumDto, Album, CreateAlbumDto, UpdateAlbumDto};

pub async fn list_albums(db: &PgPool, owner_id: Uuid) -> Result<Vec<Album>> {
    let albums = sqlx::query_as::<_, Album>(
        r#"SELECT a.*,
              COALESCE((SELECT COUNT(*) FROM photos.album_photos ap WHERE ap.album_id = a.id), 0) AS photo_count
           FROM photos.albums a
           WHERE a.owner_id = $1
           ORDER BY a.updated_at DESC"#,
    )
    .bind(owner_id)
    .fetch_all(db)
    .await
    .context("list_albums")?;

    Ok(albums)
}

pub async fn get_album(db: &PgPool, id: Uuid, owner_id: Uuid) -> Result<Option<Album>> {
    let album = sqlx::query_as::<_, Album>(
        r#"SELECT a.*,
              COALESCE((SELECT COUNT(*) FROM photos.album_photos ap WHERE ap.album_id = a.id), 0) AS photo_count
           FROM photos.albums a
           WHERE a.id = $1 AND a.owner_id = $2"#,
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .context("get_album")?;

    Ok(album)
}

pub async fn create_album(db: &PgPool, owner_id: Uuid, dto: CreateAlbumDto) -> Result<Album> {
    let album = sqlx::query_as::<_, Album>(
        r#"WITH ins AS (
               INSERT INTO photos.albums (owner_id, name, description)
               VALUES ($1, $2, $3)
               RETURNING *
           )
           SELECT ins.*, 0::bigint AS photo_count FROM ins"#,
    )
    .bind(owner_id)
    .bind(&dto.name)
    .bind(dto.description.as_deref())
    .fetch_one(db)
    .await
    .context("create_album")?;

    Ok(album)
}

pub async fn update_album(
    db: &PgPool,
    id: Uuid,
    owner_id: Uuid,
    dto: UpdateAlbumDto,
) -> Result<Option<Album>> {
    let album = sqlx::query_as::<_, Album>(
        r#"WITH upd AS (
               UPDATE photos.albums
               SET name           = COALESCE($1, name),
                   description    = COALESCE($2, description),
                   cover_photo_id = COALESCE($3, cover_photo_id),
                   updated_at     = NOW()
               WHERE id = $4 AND owner_id = $5
               RETURNING *
           )
           SELECT upd.*,
               COALESCE((SELECT COUNT(*) FROM photos.album_photos ap WHERE ap.album_id = upd.id), 0) AS photo_count
           FROM upd"#,
    )
    .bind(dto.name.as_deref())
    .bind(dto.description.as_deref())
    .bind(dto.cover_photo_id)
    .bind(id)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .context("update_album")?;

    Ok(album)
}

pub async fn delete_album(db: &PgPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let rows = sqlx::query(
        "DELETE FROM photos.albums WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .execute(db)
    .await
    .context("delete_album")?
    .rows_affected();

    Ok(rows > 0)
}

pub async fn add_photos(
    db: &PgPool,
    album_id: Uuid,
    owner_id: Uuid,
    dto: AddPhotosToAlbumDto,
) -> Result<usize> {
    // Vérifier que l'album appartient bien à l'utilisateur
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM photos.albums WHERE id = $1 AND owner_id = $2)",
    )
    .bind(album_id)
    .bind(owner_id)
    .fetch_one(db)
    .await
    .context("add_photos: check album ownership")?;

    if !exists {
        return Ok(0);
    }

    let mut count = 0usize;
    for photo_id in &dto.photo_ids {
        let rows = sqlx::query(
            "INSERT INTO photos.album_photos (album_id, photo_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(album_id)
        .bind(photo_id)
        .execute(db)
        .await
        .context("add_photos insert")?
        .rows_affected();
        count += rows as usize;
    }

    // Mettre à jour updated_at de l'album
    sqlx::query("UPDATE photos.albums SET updated_at = NOW() WHERE id = $1")
        .bind(album_id)
        .execute(db)
        .await
        .context("add_photos: update album")?;

    Ok(count)
}

pub async fn remove_photo(
    db: &PgPool,
    album_id: Uuid,
    photo_id: Uuid,
    owner_id: Uuid,
) -> Result<bool> {
    let rows = sqlx::query(
        r#"DELETE FROM photos.album_photos
           WHERE album_id = $1 AND photo_id = $2
             AND (SELECT owner_id FROM photos.albums WHERE id = $1) = $3"#,
    )
    .bind(album_id)
    .bind(photo_id)
    .bind(owner_id)
    .execute(db)
    .await
    .context("remove_photo from album")?
    .rows_affected();

    Ok(rows > 0)
}
