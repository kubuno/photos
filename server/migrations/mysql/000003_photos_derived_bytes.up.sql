-- Weight of the derivatives (thumbnail + preview) generated for a photo. See the
-- PostgreSQL migration of the same name for the full rationale.
--
-- MySQL has no partial index, so the pending-backfill scan is served by a plain
-- composite index over the same columns.

ALTER TABLE photos.photos
    ADD COLUMN derived_bytes BIGINT NOT NULL DEFAULT 0;

CREATE INDEX idx_photos_derived_pending
    ON photos.photos (derived_bytes, has_thumbnail, has_preview);
