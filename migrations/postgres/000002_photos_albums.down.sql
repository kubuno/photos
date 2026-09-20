ALTER TABLE photos.shares DROP CONSTRAINT IF EXISTS fk_shares_album;
DROP TABLE IF EXISTS photos.album_photos;
DROP TABLE IF EXISTS photos.albums;
