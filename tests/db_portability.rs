//! Runs photos' own migrations and its own photo/album services against a real
//! server of **each** engine, from a single compiled binary — the proof that the
//! engine is a run-time choice, not a build-time one.
//!
//! * SQLite always runs (a temp file, no server).
//! * PostgreSQL runs when `KUBUNO_PG_TEST_URL` points at a throwaway database.
//! * MySQL/MariaDB runs when `KUBUNO_MYSQL_TEST_URL` does.
//!
//! ```sh
//! KUBUNO_PG_TEST_URL=postgres://u:p@localhost:5433/photos \
//! KUBUNO_MYSQL_TEST_URL=mysql://root@127.0.0.1:3307/photos \
//!   cargo test --test db_portability
//! ```
//!
//! The same binary contains all three drivers; each engine's suite is one test.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use kubuno_db::{params, DbPool};
use kubuno_photos::models::{
    AddPhotosToAlbumDto, CreateAlbumDto, ListPhotosQuery, Share, UpdateAlbumDto, UpdatePhotoDto,
};
use kubuno_photos::services::album_service::{
    add_photos, create_album, delete_album, get_album, list_albums, remove_photo, update_album,
};
use kubuno_photos::services::photo_service::{
    delete_photo, get_photo, list_photos, restore_photo, trash_photo, update_photo,
};
use kubuno_photos::SCHEMA;
use kubuno_storage::{LocalStorage, StorageBackend};
use uuid::Uuid;

fn base_settings(engine: &str) -> kubuno_db::DbSettings {
    kubuno_db::DbSettings {
        engine: engine.to_string(),
        url: None,
        host: None,
        port: None,
        user: None,
        password: None,
        database: None,
        path: None,
        max_connections: 4,
        min_connections: 0,
        connect_timeout: std::time::Duration::from_secs(10),
        run_migrations: true,
        schema_prefix: None,
    }
}

/// Migrations only run one at a time: the PostgreSQL and MySQL suites may share a
/// server.
static EXCLUSIVE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn migrated_pool(settings: kubuno_db::DbSettings) -> (DbPool, impl Sized) {
    let guard = EXCLUSIVE.lock().await;
    let pool = kubuno_db::connect(&settings, SCHEMA).await.expect("connect");

    // Exactly the calls `main.rs` makes.
    kubuno_db::migrations!(
        "./migrations/postgres",
        "./migrations/mysql",
        "./migrations/sqlite",
    )
    .run(&pool, SCHEMA)
    .await
    .expect("migrations");

    // Harmless where photos never uses the outbox, but proves the DDL is valid on
    // every engine.
    kubuno_db::events::ensure_outbox(&pool, SCHEMA)
        .await
        .expect("outbox");

    (pool, guard)
}

fn list_query() -> ListPhotosQuery {
    ListPhotosQuery {
        album_id: None,
        starred: None,
        trashed: None,
        from: None,
        to: None,
        search: None,
        limit: None,
        offset: None,
    }
}

/// Inserts a photo row directly, mirroring `upload_photo`'s statement but without
/// the storage backend or image decoding: this test exercises the SQL layer, not
/// the codecs.
#[allow(clippy::too_many_arguments)]
async fn seed_photo(
    pool: &DbPool,
    owner: Uuid,
    name: &str,
    size: i64,
    trashed: bool,
    starred: bool,
    has_thumb: bool,
    has_prev: bool,
    derived: i64,
    taken_at: Option<DateTime<Utc>>,
) -> Uuid {
    let id = kubuno_db::new_id();
    let trashed_at: Option<DateTime<Utc>> = if trashed { Some(Utc::now()) } else { None };
    pool.execute(
        r#"INSERT INTO photos.photos
             (id, owner_id, filename, original_name, mime_type, size_bytes, storage_path,
              has_thumbnail, has_preview, is_starred, is_trashed, trashed_at, taken_at,
              derived_bytes, metadata)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)"#,
        params![
            id,
            owner,
            name,
            name,
            "image/jpeg",
            size,
            format!("photos/{owner}/{id}.jpg"),
            has_thumb,
            has_prev,
            starred,
            trashed,
            trashed_at,
            taken_at,
            derived,
            serde_json::json!({}),
        ],
    )
    .await
    .expect("seed photo");
    id
}

/// Everything the handlers exercise: the read/update/trash/restore/delete path,
/// the album path, a public-share round-trip and the usage aggregate — the one
/// whose `FILTER`/cast rewrite has to decode on every engine.
async fn full_suite(pool: &DbPool) {
    let dir = tempfile::tempdir().expect("tempdir");
    let storage: Arc<dyn StorageBackend> = Arc::new(
        LocalStorage::new(dir.path().to_str().expect("path"))
            .await
            .expect("storage"),
    );

    let owner = Uuid::new_v4();
    let taken = Utc::now();

    // 2 live (one starred, one with both derivatives), 1 trashed.
    let live = seed_photo(pool, owner, "a.jpg", 1000, false, false, true, true, 300, Some(taken)).await;
    let starred = seed_photo(pool, owner, "b.jpg", 2000, false, true, true, false, 100, None).await;
    let trashed = seed_photo(pool, owner, "c.jpg", 4000, true, false, false, false, 0, None).await;

    // ── get_photo, with owner scoping ──
    let got = get_photo(pool, live, owner).await.unwrap().expect("photo");
    assert_eq!(got.size_bytes, 1000);
    assert_eq!(got.original_name, "a.jpg");
    assert!(got.has_thumbnail && got.has_preview);
    assert_eq!(got.derived_bytes, 300);
    assert!(
        get_photo(pool, live, Uuid::new_v4()).await.unwrap().is_none(),
        "a photo is scoped to its owner"
    );

    // ── the usage aggregate: FILTER rewritten to CASE, decoded as i64 ──
    let backend = pool.backend();
    let thumb_count = backend.count_bigint("CASE WHEN has_thumbnail THEN 1 END");
    let prev_count = backend.count_bigint("CASE WHEN has_preview THEN 1 END");
    let dc = format!("{thumb_count} + {prev_count}");
    let sql = format!(
        "SELECT owner_id, {cb}, {cc}, {tb}, {tc}, {db_bytes}, {dc} \
         FROM photos.photos WHERE owner_id = $1 GROUP BY owner_id",
        cb = backend.sum_bigint("CASE WHEN NOT is_trashed THEN size_bytes ELSE 0 END"),
        cc = backend.count_bigint("CASE WHEN NOT is_trashed THEN 1 END"),
        tb = backend.sum_bigint("CASE WHEN is_trashed THEN size_bytes ELSE 0 END"),
        tc = backend.count_bigint("CASE WHEN is_trashed THEN 1 END"),
        db_bytes = backend.sum_bigint("derived_bytes"),
    );
    let rows: Vec<(Uuid, i64, i64, i64, i64, i64, i64)> =
        pool.fetch_all_as(&sql, params![owner]).await.expect("usage aggregate");
    // content 1000+2000 / 2 photos, trash 4000 / 1 photo, derived 400 / 3 files.
    assert_eq!(rows, vec![(owner, 3000, 2, 4000, 1, 400, 3)]);

    // ── a public share round-trip (INSERT + RETURNING-less reselect shape) ──
    let share_id = kubuno_db::new_id();
    pool.execute(
        "INSERT INTO photos.shares (id, owner_id, photo_id, token, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
        params![share_id, owner, live, "tok-123", None::<DateTime<Utc>>],
    )
    .await
    .expect("insert share");
    let share: Share = pool
        .fetch_one_as("SELECT * FROM photos.shares WHERE id = $1", params![share_id])
        .await
        .expect("reselect share");
    assert_eq!(share.token, "tok-123");
    assert_eq!(share.photo_id, Some(live));

    // ── list: default (live only), starred, trashed ──
    let all_live = list_photos(pool, owner, list_query()).await.unwrap();
    assert_eq!(all_live.len(), 2, "default list is the live photos");
    assert!(all_live.iter().all(|p| !p.is_trashed));

    let only_starred =
        list_photos(pool, owner, ListPhotosQuery { starred: Some(true), ..list_query() }).await.unwrap();
    assert_eq!(only_starred.len(), 1);
    assert_eq!(only_starred[0].id, starred);

    let only_trashed =
        list_photos(pool, owner, ListPhotosQuery { trashed: Some(true), ..list_query() }).await.unwrap();
    assert_eq!(only_trashed.len(), 1);
    assert_eq!(only_trashed[0].id, trashed);

    // ── search (bound `%pattern%`, ILIKE per engine) ──
    let hits = list_photos(
        pool,
        owner,
        ListPhotosQuery { search: Some("a.jpg".into()), ..list_query() },
    )
    .await
    .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, live);

    // ── date filter: `from` excludes the NULL-taken_at row ──
    let since = list_photos(
        pool,
        owner,
        ListPhotosQuery { from: Some(taken - chrono::Duration::hours(1)), ..list_query() },
    )
    .await
    .unwrap();
    assert_eq!(since.len(), 1, "only the row with a taken_at passes `from`");
    assert_eq!(since[0].id, live);

    // ── update_photo ──
    let updated = update_photo(
        pool,
        live,
        owner,
        UpdatePhotoDto { description: Some("hello".into()), is_starred: Some(true), taken_at: None },
    )
    .await
    .unwrap()
    .expect("updated");
    assert_eq!(updated.description.as_deref(), Some("hello"));
    assert!(updated.is_starred);
    assert!(update_photo(pool, live, Uuid::new_v4(), UpdatePhotoDto { description: None, is_starred: None, taken_at: None })
        .await
        .unwrap()
        .is_none(), "a wrong owner updates nothing");

    // ── trash / restore ──
    assert!(trash_photo(pool, live, owner).await.unwrap());
    assert!(get_photo(pool, live, owner).await.unwrap().unwrap().is_trashed);
    assert!(!trash_photo(pool, live, owner).await.unwrap(), "already trashed");
    assert!(restore_photo(pool, live, owner).await.unwrap());
    assert!(!get_photo(pool, live, owner).await.unwrap().unwrap().is_trashed);

    // ── albums ──
    let album = create_album(
        pool,
        owner,
        CreateAlbumDto { name: "Trip".into(), description: Some("desc".into()) },
    )
    .await
    .unwrap();
    assert_eq!(album.name, "Trip");
    assert_eq!(album.photo_count, 0, "a fresh album is empty");

    assert_eq!(list_albums(pool, owner).await.unwrap().len(), 1);
    assert!(get_album(pool, album.id, owner).await.unwrap().is_some());

    let added = add_photos(
        pool,
        album.id,
        owner,
        AddPhotosToAlbumDto { photo_ids: vec![live, starred] },
    )
    .await
    .unwrap();
    assert_eq!(added, 2);
    // The upsert-ignore must not double-count a second add.
    let again = add_photos(
        pool,
        album.id,
        owner,
        AddPhotosToAlbumDto { photo_ids: vec![live] },
    )
    .await
    .unwrap();
    assert_eq!(again, 0, "ON CONFLICT DO NOTHING inserts nothing the second time");

    assert_eq!(get_album(pool, album.id, owner).await.unwrap().unwrap().photo_count, 2);

    let in_album =
        list_photos(pool, owner, ListPhotosQuery { album_id: Some(album.id), ..list_query() }).await.unwrap();
    assert_eq!(in_album.len(), 2);

    assert!(remove_photo(pool, album.id, starred, owner).await.unwrap());
    assert_eq!(get_album(pool, album.id, owner).await.unwrap().unwrap().photo_count, 1);

    let renamed = update_album(
        pool,
        album.id,
        owner,
        UpdateAlbumDto { name: Some("Trip 2".into()), description: None, cover_photo_id: Some(live) },
    )
    .await
    .unwrap()
    .expect("renamed");
    assert_eq!(renamed.name, "Trip 2");
    assert_eq!(renamed.cover_photo_id, Some(live));
    assert_eq!(renamed.photo_count, 1);

    assert!(delete_album(pool, album.id, owner).await.unwrap());
    assert!(get_album(pool, album.id, owner).await.unwrap().is_none());

    // ── hard delete (read-before-write on a RETURNING-less engine) ──
    assert!(delete_photo(pool, storage.as_ref(), trashed, owner).await.unwrap());
    assert!(get_photo(pool, trashed, owner).await.unwrap().is_none());
    assert!(
        !delete_photo(pool, storage.as_ref(), Uuid::new_v4(), owner).await.unwrap(),
        "deleting a missing photo is Ok(false)"
    );
}

#[tokio::test]
async fn sqlite_from_the_one_binary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut s = base_settings("sqlite");
    s.path = Some(dir.path().to_string_lossy().into_owned());
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}

#[tokio::test]
async fn postgres_from_the_one_binary() {
    let Ok(url) = std::env::var("KUBUNO_PG_TEST_URL") else {
        eprintln!("skipping: KUBUNO_PG_TEST_URL not set");
        return;
    };
    let mut s = base_settings("postgres");
    s.url = Some(url);
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}

#[tokio::test]
async fn mysql_from_the_one_binary() {
    let Ok(url) = std::env::var("KUBUNO_MYSQL_TEST_URL") else {
        eprintln!("skipping: KUBUNO_MYSQL_TEST_URL not set");
        return;
    };
    let mut s = base_settings("mysql");
    s.url = Some(url);
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}
