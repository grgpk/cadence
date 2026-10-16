-- A host's booking page: what belongs to the host, whatever is booked.
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

-- The kinds of call a host offers on their page. The oldest active one is
-- the host's first: it is the one a plan with a single type books.
CREATE TABLE event_types (
    unid                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_unid               UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    -- The type's part of the link: /book/<public_slug>/<slug>.
    slug                    TEXT NOT NULL,
    title                   TEXT NOT NULL DEFAULT 'Book your call',
    description             TEXT NOT NULL DEFAULT 'Choose a time that works for you.',
    -- NULL follows the slot length of the host's hours.
    duration_minutes        INT,
    -- Closes times next to a booked call. Hosts start without one.
    buffer_minutes          INT NOT NULL DEFAULT 0,
    minimum_notice_hours    INT NOT NULL DEFAULT 4,
    -- How far ahead a visitor can book.
    booking_window_days     INT NOT NULL DEFAULT 7,
    -- Booking form questions, all hidden until the host turns them on.
    -- Names follow the `leads` columns they gate.
    ask_currently_working_on BOOLEAN NOT NULL DEFAULT false,
    ask_how_heard_about_us  BOOLEAN NOT NULL DEFAULT false,
    ask_what_stopping_you   BOOLEAN NOT NULL DEFAULT false,
    ask_investment_comfort  BOOLEAN NOT NULL DEFAULT false,
    ask_urgency_level       BOOLEAN NOT NULL DEFAULT false,
    is_active               BOOLEAN NOT NULL DEFAULT true,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT event_types_slug_unique UNIQUE (host_unid, slug),
    CONSTRAINT event_types_slug_format CHECK (
        slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$' AND char_length(slug) BETWEEN 2 AND 40
    ),
    CONSTRAINT event_types_duration CHECK (duration_minutes BETWEEN 5 AND 240),
    CONSTRAINT event_types_buffer CHECK (buffer_minutes BETWEEN 0 AND 240),
    CONSTRAINT event_types_notice CHECK (minimum_notice_hours BETWEEN 0 AND 720),
    CONSTRAINT event_types_booking_window CHECK (booking_window_days BETWEEN 1 AND 90)
);

CREATE INDEX event_types_host_idx ON event_types (host_unid, created_at);
