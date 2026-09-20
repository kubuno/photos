-- Weight of the derivatives (thumbnail + preview) generated for a photo. See the
-- PostgreSQL migration of the same name for the full rationale.

ALTER TABLE photos.photos ADD COLUMN derived_bytes INTEGER NOT NULL DEFAULT 0;

CREATE INDEX photos.idx_photos_derived_pending
    ON photos (id)
 WHERE derived_bytes = 0 AND (has_thumbnail OR has_preview);
