-- Sessions carry the version they were issued under. Changing or resetting a
-- password raises it, which signs every older session out.
ALTER TABLE users ADD COLUMN session_version INT NOT NULL DEFAULT 0;
