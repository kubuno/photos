-- Albums et association photos↔albums (MySQL / MariaDB).

CREATE TABLE photos.albums (
    id              BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id        BINARY(16)   NOT NULL,
    name            VARCHAR(255) NOT NULL,
    description     TEXT         NULL,
    cover_photo_id  BINARY(16)   NULL,
    is_shared       BOOLEAN      NOT NULL DEFAULT FALSE,
    share_token     VARCHAR(64)  NULL UNIQUE,
    created_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                                 ON UPDATE CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_albums_cover FOREIGN KEY (cover_photo_id)
        REFERENCES photos.photos(id) ON DELETE SET NULL
);

CREATE INDEX idx_albums_owner ON photos.albums(owner_id);

CREATE TABLE photos.album_photos (
    album_id    BINARY(16)  NOT NULL,
    photo_id    BINARY(16)  NOT NULL,
    added_at    DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (album_id, photo_id),
    CONSTRAINT fk_album_photos_album FOREIGN KEY (album_id)
        REFERENCES photos.albums(id) ON DELETE CASCADE,
    CONSTRAINT fk_album_photos_photo FOREIGN KEY (photo_id)
        REFERENCES photos.photos(id) ON DELETE CASCADE
);

CREATE INDEX idx_album_photos_photo ON photos.album_photos(photo_id);

ALTER TABLE photos.shares
    ADD CONSTRAINT fk_shares_album FOREIGN KEY (album_id)
        REFERENCES photos.albums(id) ON DELETE CASCADE;
