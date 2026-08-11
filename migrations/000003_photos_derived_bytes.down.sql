DROP INDEX IF EXISTS photos.idx_photos_derived_pending;

ALTER TABLE photos.photos
    DROP COLUMN IF EXISTS derived_bytes;
