-- Module Photos — schéma principal (MySQL / MariaDB).
--
-- The `photos` database is created by kubuno-db's `ensure_schema` before the
-- migrator runs, so there is no CREATE DATABASE here.
--
-- Differences from the PostgreSQL file, and why:
--   * UUID -> BINARY(16): what sqlx encodes a `uuid::Uuid` as on MySQL.
--   * No DEFAULT on `id`: MySQL has no gen_random_uuid(), and the process has to
--     know the key anyway since MySQL has no RETURNING.
--   * TIMESTAMPTZ -> DATETIME(6): no time zone in the column; every value
--     written is UTC (the session runs at time_zone '+00:00').
--   * JSONB -> JSON; DOUBLE PRECISION -> DOUBLE.
--   * The updated_at trigger becomes ON UPDATE CURRENT_TIMESTAMP(6).
--   * Partial indexes (WHERE …) have no MySQL form; the plain index is kept.
--   * The share-target CHECK is dropped (enforced in the handler); MySQL/MariaDB
--     support for it is uneven and it is defence-in-depth only.
CREATE TABLE photos.photos (
    id              BINARY(16)    NOT NULL PRIMARY KEY,
    owner_id        BINARY(16)    NOT NULL,
    filename        VARCHAR(500)  NOT NULL,
    original_name   VARCHAR(500)  NOT NULL,
    mime_type       VARCHAR(100)  NOT NULL,
    size_bytes      BIGINT        NOT NULL,
    width           INT           NULL,
    height          INT           NULL,
    storage_path    VARCHAR(1000) NOT NULL,
    content_hash    VARCHAR(64)   NULL,
    taken_at        DATETIME(6)   NULL,
    camera_make     VARCHAR(100)  NULL,
    camera_model    VARCHAR(100)  NULL,
    gps_lat         DOUBLE        NULL,
    gps_lon         DOUBLE        NULL,
    has_thumbnail   BOOLEAN       NOT NULL DEFAULT FALSE,
    has_preview     BOOLEAN       NOT NULL DEFAULT FALSE,
    is_starred      BOOLEAN       NOT NULL DEFAULT FALSE,
    is_trashed      BOOLEAN       NOT NULL DEFAULT FALSE,
    trashed_at      DATETIME(6)   NULL,
    description     TEXT          NULL,
    metadata        JSON          NOT NULL,
    created_at      DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at      DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                                  ON UPDATE CURRENT_TIMESTAMP(6)
);

CREATE INDEX idx_photos_owner   ON photos.photos(owner_id);
CREATE INDEX idx_photos_taken   ON photos.photos(owner_id, taken_at);
CREATE INDEX idx_photos_starred ON photos.photos(owner_id, is_starred);
CREATE INDEX idx_photos_trashed ON photos.photos(owner_id, is_trashed);
CREATE INDEX idx_photos_hash    ON photos.photos(content_hash);

CREATE TABLE photos.shares (
    id          BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id    BINARY(16)  NOT NULL,
    photo_id    BINARY(16)  NULL,
    album_id    BINARY(16)  NULL,  -- FK ajoutée dans migration 000002
    token       VARCHAR(64) NOT NULL UNIQUE,
    expires_at  DATETIME(6) NULL,
    created_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_shares_photo FOREIGN KEY (photo_id)
        REFERENCES photos.photos(id) ON DELETE CASCADE
);

CREATE INDEX idx_photos_shares_owner ON photos.shares(owner_id);
