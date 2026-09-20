-- Weight of the derivatives (thumbnail + preview) generated for a photo.
--
-- The originals are already measured by `size_bytes`, but the two JPEGs written
-- next to them — `photos/{owner}/thumbs/{id}.jpg` and
-- `photos/{owner}/previews/{id}.jpg` — occupied real space that nothing recorded:
-- `has_thumbnail` / `has_preview` are booleans and say only whether the files
-- exist. Without this column the module could not declare its `thumbnails`
-- category without stat-ing the storage backend once per photo on every sync.
--
-- Written by the derivative generator (upload, and regeneration after a
-- transform). Existing rows start at 0 and are backfilled in bounded batches by
-- the usage reporter, which measures them through the storage backend and writes
-- the result once.
--
-- 0 is the "not measured yet" sentinel *and* the correct value for a photo with
-- no derivatives; the backfill disambiguates by also clearing `has_thumbnail` /
-- `has_preview` when the files turn out not to exist, so a row can never be
-- re-measured forever.

ALTER TABLE photos.photos
    ADD COLUMN IF NOT EXISTS derived_bytes BIGINT NOT NULL DEFAULT 0;

-- Partial index for the backfill scan: it looks only for rows that claim a
-- derivative but carry no measurement yet, and that set empties as it works.
CREATE INDEX IF NOT EXISTS idx_photos_derived_pending
    ON photos.photos (id)
 WHERE derived_bytes = 0 AND (has_thumbnail OR has_preview);
