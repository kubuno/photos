-- Albums et association photos↔albums (PostgreSQL).

CREATE TABLE IF NOT EXISTS photos.albums (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id        UUID NOT NULL,
    name            VARCHAR(255) NOT NULL,
    description     TEXT,
    cover_photo_id  UUID REFERENCES photos.photos(id) ON DELETE SET NULL,
    is_shared       BOOLEAN NOT NULL DEFAULT FALSE,
    share_token     VARCHAR(64) UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_albums_owner ON photos.albums(owner_id);

CREATE TRIGGER albums_updated_at
    BEFORE UPDATE ON photos.albums
    FOR EACH ROW EXECUTE FUNCTION photos.set_photos_updated_at();

CREATE TABLE IF NOT EXISTS photos.album_photos (
    album_id    UUID NOT NULL REFERENCES photos.albums(id) ON DELETE CASCADE,
    photo_id    UUID NOT NULL REFERENCES photos.photos(id) ON DELETE CASCADE,
    added_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (album_id, photo_id)
);

CREATE INDEX IF NOT EXISTS idx_album_photos_photo ON photos.album_photos(photo_id);

-- Ajouter la FK album_id dans shares maintenant que la table albums existe
ALTER TABLE photos.shares
    ADD CONSTRAINT fk_shares_album
    FOREIGN KEY (album_id) REFERENCES photos.albums(id) ON DELETE CASCADE;
