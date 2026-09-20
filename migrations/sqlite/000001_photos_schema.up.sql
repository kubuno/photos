-- Module Photos — schéma principal (SQLite).
--
-- `photos` is an ATTACHed database file, attached on every pooled connection by
-- kubuno-db, so the qualified names below resolve as they do on the other two
-- engines.
--
-- Differences from the PostgreSQL file, and why:
--   * UUID -> BLOB: what sqlx encodes a `uuid::Uuid` as on SQLite.
--   * No DEFAULT on `id`: SQLite has no UUID generator; the process supplies it.
--   * TIMESTAMPTZ -> TEXT, in the `%F %T%.f` shape sqlx decodes into
--     DateTime<Utc>. Every value written is UTC.
--   * BOOLEAN -> INTEGER (0/1); JSONB -> TEXT; DOUBLE PRECISION -> REAL.
--   * The updated_at trigger is written by hand; it does not recurse because
--     SQLite leaves recursive_triggers off.
--   * The share-target CHECK is dropped (enforced in the handler).
CREATE TABLE photos.photos (
    id              BLOB    NOT NULL PRIMARY KEY,
    owner_id        BLOB    NOT NULL,
    filename        TEXT    NOT NULL,
    original_name   TEXT    NOT NULL,
    mime_type       TEXT    NOT NULL,
    size_bytes      INTEGER NOT NULL,
    width           INTEGER,
    height          INTEGER,
    storage_path    TEXT    NOT NULL,
    content_hash    TEXT,
    taken_at        TEXT,
    camera_make     TEXT,
    camera_model    TEXT,
    gps_lat         REAL,
    gps_lon         REAL,
    has_thumbnail   INTEGER NOT NULL DEFAULT 0,
    has_preview     INTEGER NOT NULL DEFAULT 0,
    is_starred      INTEGER NOT NULL DEFAULT 0,
    is_trashed      INTEGER NOT NULL DEFAULT 0,
    trashed_at      TEXT,
    description     TEXT,
    metadata        TEXT    NOT NULL DEFAULT '{}',
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE INDEX photos.idx_photos_owner   ON photos(owner_id);
CREATE INDEX photos.idx_photos_taken   ON photos(owner_id, taken_at DESC);
CREATE INDEX photos.idx_photos_starred ON photos(owner_id, is_starred) WHERE is_starred = 1;
CREATE INDEX photos.idx_photos_trashed ON photos(owner_id, is_trashed);
CREATE INDEX photos.idx_photos_hash    ON photos(content_hash) WHERE content_hash IS NOT NULL;

CREATE TRIGGER photos.photos_updated_at AFTER UPDATE ON photos
BEGIN
    UPDATE photos SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id;
END;

CREATE TABLE photos.shares (
    id          BLOB NOT NULL PRIMARY KEY,
    owner_id    BLOB NOT NULL,
    photo_id    BLOB REFERENCES photos(id) ON DELETE CASCADE,
    album_id    BLOB,
    token       TEXT NOT NULL UNIQUE,
    expires_at  TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE INDEX photos.idx_photos_shares_owner ON shares(owner_id);
