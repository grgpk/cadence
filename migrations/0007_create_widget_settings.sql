CREATE TABLE widget_settings (
    host_unid               UUID PRIMARY KEY REFERENCES users(unid) ON DELETE CASCADE,
    public_slug             TEXT NOT NULL UNIQUE,
    title                   TEXT NOT NULL DEFAULT 'Book your call',
    description             TEXT NOT NULL DEFAULT 'Choose a time that works for you.',
    accent_color            TEXT NOT NULL DEFAULT '#111111',
    duration_minutes        INT NOT NULL DEFAULT 30,
    -- Closes times next to a booked call. Hosts start without one.
    buffer_minutes          INT NOT NULL DEFAULT 0,
    minimum_notice_hours    INT NOT NULL DEFAULT 4,
    -- How far ahead a visitor can book.
    booking_window_days     INT NOT NULL DEFAULT 7,
    timezone                TEXT NOT NULL DEFAULT 'Europe/Paris',
    is_active               BOOLEAN NOT NULL DEFAULT true,
    -- A host's own place to meet (Zoom, a personal Meet room, any https
    -- link), used when Google Calendar does not create a Meet link.
    meeting_link            TEXT,
    -- Booking form questions, all hidden until the host turns them on.
    -- Names follow the `leads` columns they gate.
    ask_currently_working_on BOOLEAN NOT NULL DEFAULT false,
    ask_how_heard_about_us  BOOLEAN NOT NULL DEFAULT false,
    ask_what_stopping_you   BOOLEAN NOT NULL DEFAULT false,
    ask_investment_comfort  BOOLEAN NOT NULL DEFAULT false,
    ask_urgency_level       BOOLEAN NOT NULL DEFAULT false,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT widget_duration CHECK (duration_minutes BETWEEN 5 AND 240),
    CONSTRAINT widget_buffer CHECK (buffer_minutes BETWEEN 0 AND 240),
    CONSTRAINT widget_notice CHECK (minimum_notice_hours BETWEEN 0 AND 720),
    CONSTRAINT widget_booking_window CHECK (booking_window_days BETWEEN 1 AND 90),
    CONSTRAINT widget_public_slug_format CHECK (
        public_slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'
        AND char_length(public_slug) BETWEEN 3 AND 40
    )
);
