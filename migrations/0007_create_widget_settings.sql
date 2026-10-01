CREATE TABLE widget_settings (
    host_unid               UUID PRIMARY KEY REFERENCES users(unid) ON DELETE CASCADE,
    public_slug             TEXT NOT NULL UNIQUE,
    title                   TEXT NOT NULL DEFAULT 'Book your call',
    description             TEXT NOT NULL DEFAULT 'Choose a time that works for you.',
    accent_color            TEXT NOT NULL DEFAULT '#111111',
    duration_minutes        INT NOT NULL DEFAULT 30,
    buffer_minutes          INT NOT NULL DEFAULT 15,
    minimum_notice_hours    INT NOT NULL DEFAULT 4,
    timezone                TEXT NOT NULL DEFAULT 'Europe/Paris',
    is_active               BOOLEAN NOT NULL DEFAULT true,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT widget_duration CHECK (duration_minutes BETWEEN 5 AND 240),
    CONSTRAINT widget_buffer CHECK (buffer_minutes BETWEEN 0 AND 240),
    CONSTRAINT widget_notice CHECK (minimum_notice_hours BETWEEN 0 AND 720)
);
