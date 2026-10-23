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

-- Booking form questions a type asks. None are asked until the host turns
-- them on.
CREATE TABLE event_type_questions (
    event_type_unid UUID NOT NULL REFERENCES event_types(unid) ON DELETE CASCADE,
    question        TEXT NOT NULL,
    PRIMARY KEY (event_type_unid, question),
    CONSTRAINT event_type_questions_question CHECK (question IN (
        'currently_working_on', 'how_heard_about_us', 'what_stopping_you',
        'investment_comfort', 'urgency_level'
    ))
);
