DROP INDEX idx_photos_derived_pending ON photos.photos;

ALTER TABLE photos.photos
    DROP COLUMN derived_bytes;
