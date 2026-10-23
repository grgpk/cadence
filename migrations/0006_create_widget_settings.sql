CREATE TABLE widget_settings (
    host_unid               UUID PRIMARY KEY REFERENCES users(unid) ON DELETE CASCADE,
    public_slug             TEXT NOT NULL UNIQUE,
    accent_color            TEXT NOT NULL DEFAULT '#111111',
    timezone                TEXT NOT NULL DEFAULT 'Europe/Paris',
    is_active               BOOLEAN NOT NULL DEFAULT true,
    -- A host's own place to meet (Zoom, a personal Meet room, any https
    -- link), used when Google Calendar does not create a Meet link.
    meeting_link            TEXT,
    -- Where every call happens: a Google Meet made with the calendar event,
    -- or the host's own `meeting_link`.
    conferencing_app        TEXT NOT NULL DEFAULT 'google_meet',
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT widget_conferencing_app CHECK (conferencing_app IN ('google_meet', 'custom_link')),
    CONSTRAINT widget_public_slug_format CHECK (
        public_slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'
        AND char_length(public_slug) BETWEEN 3 AND 40
    )
);
