-- Module Photos — schéma principal (PostgreSQL).
--
-- Schema-qualified throughout: kubuno-db's migrator runs on the pool without a
-- schema search_path, so every object names `photos.` explicitly. `uuid-ossp`
-- (uuid_generate_v4) is created by the core; the DEFAULT is overridden anyway,
-- as the process now generates every key in Rust and binds it.
CREATE SCHEMA IF NOT EXISTS photos;

CREATE TABLE IF NOT EXISTS photos.photos (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id        UUID NOT NULL,
    filename        VARCHAR(500) NOT NULL,
    original_name   VARCHAR(500) NOT NULL,
    mime_type       VARCHAR(100) NOT NULL,
    size_bytes      BIGINT NOT NULL,
    width           INTEGER,
    height          INTEGER,
    storage_path    VARCHAR(1000) NOT NULL,
    content_hash    VARCHAR(64),
    -- EXIF
    taken_at        TIMESTAMPTZ,
    camera_make     VARCHAR(100),
    camera_model    VARCHAR(100),
    gps_lat         DOUBLE PRECISION,
    gps_lon         DOUBLE PRECISION,
    -- État
    has_thumbnail   BOOLEAN NOT NULL DEFAULT FALSE,
    has_preview     BOOLEAN NOT NULL DEFAULT FALSE,
    is_starred      BOOLEAN NOT NULL DEFAULT FALSE,
    is_trashed      BOOLEAN NOT NULL DEFAULT FALSE,
    trashed_at      TIMESTAMPTZ,
    description     TEXT,
    metadata        JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_photos_owner   ON photos.photos(owner_id);
CREATE INDEX IF NOT EXISTS idx_photos_taken   ON photos.photos(owner_id, taken_at DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_photos_starred ON photos.photos(owner_id, is_starred) WHERE is_starred = TRUE;
CREATE INDEX IF NOT EXISTS idx_photos_trashed ON photos.photos(owner_id, is_trashed);
CREATE INDEX IF NOT EXISTS idx_photos_hash    ON photos.photos(content_hash) WHERE content_hash IS NOT NULL;

CREATE OR REPLACE FUNCTION photos.set_photos_updated_at()
RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = NOW(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER photos_updated_at
    BEFORE UPDATE ON photos.photos
    FOR EACH ROW EXECUTE FUNCTION photos.set_photos_updated_at();

CREATE TABLE IF NOT EXISTS photos.shares (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id    UUID NOT NULL,
    photo_id    UUID REFERENCES photos.photos(id) ON DELETE CASCADE,
    album_id    UUID,  -- référence ajoutée dans migration 000002
    token       VARCHAR(64) UNIQUE NOT NULL,
    expires_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT share_target_single CHECK (
        (photo_id IS NOT NULL AND album_id IS NULL) OR
        (photo_id IS NULL AND album_id IS NOT NULL) OR
        (photo_id IS NOT NULL AND album_id IS NOT NULL) = FALSE
    )
);

CREATE INDEX IF NOT EXISTS idx_photos_shares_token    ON photos.shares(token);
CREATE INDEX IF NOT EXISTS idx_photos_shares_owner    ON photos.shares(owner_id);
