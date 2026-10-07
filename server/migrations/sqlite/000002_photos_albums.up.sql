-- Albums et association photos↔albums (SQLite).
--
-- SQLite cannot add a foreign key to an existing table (ALTER TABLE ADD
-- CONSTRAINT is unsupported), so the shares.album_id -> albums FK that the
-- PostgreSQL/MySQL migrations add here is omitted; the column stays a plain BLOB
-- and the relationship is enforced by the handler.

CREATE TABLE photos.albums (
    id              BLOB    NOT NULL PRIMARY KEY,
    owner_id        BLOB    NOT NULL,
    name            TEXT    NOT NULL,
    description     TEXT,
    cover_photo_id  BLOB    REFERENCES photos(id) ON DELETE SET NULL,
    is_shared       INTEGER NOT NULL DEFAULT 0,
    share_token     TEXT    UNIQUE,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE INDEX photos.idx_albums_owner ON albums(owner_id);

CREATE TRIGGER photos.albums_updated_at AFTER UPDATE ON albums
BEGIN
    UPDATE albums SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id;
END;

CREATE TABLE photos.album_photos (
    album_id    BLOB NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
    photo_id    BLOB NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    added_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (album_id, photo_id)
);

CREATE INDEX photos.idx_album_photos_photo ON album_photos(photo_id);
