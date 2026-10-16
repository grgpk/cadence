-- Plan 28: a host's booking page splits into what belongs to the host
-- (`widget_settings`) and the kinds of call they offer (`event_types`).
-- Every host's page becomes their first event type, with the same title,
-- description, rules and questions; every booking is linked to it.

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

-- Each page as it is today becomes its host's first type, at /book/<host>/call.
-- `duration_minutes` was never read: calls kept following the hours, so the
-- type does too (NULL).
INSERT INTO event_types (
    host_unid, slug, title, description,
    buffer_minutes, minimum_notice_hours, booking_window_days,
    ask_currently_working_on, ask_how_heard_about_us, ask_what_stopping_you,
    ask_investment_comfort, ask_urgency_level,
    created_at, updated_at
)
SELECT
    host_unid, 'call', title, description,
    buffer_minutes, minimum_notice_hours, booking_window_days,
    ask_currently_working_on, ask_how_heard_about_us, ask_what_stopping_you,
    ask_investment_comfort, ask_urgency_level,
    created_at, updated_at
FROM widget_settings;

-- The kind of call booked. A deleted type leaves its calls in place; they
-- are read with the host's first type then.
ALTER TABLE bookings
    ADD COLUMN event_type_unid UUID REFERENCES event_types(unid) ON DELETE SET NULL;

UPDATE bookings b
SET event_type_unid = e.unid
FROM event_types e
WHERE e.host_unid = b.host_unid;

-- What moved to the type leaves the page. The checks on these columns go
-- with them.
ALTER TABLE widget_settings
    DROP COLUMN title,
    DROP COLUMN description,
    DROP COLUMN duration_minutes,
    DROP COLUMN buffer_minutes,
    DROP COLUMN minimum_notice_hours,
    DROP COLUMN booking_window_days,
    DROP COLUMN ask_currently_working_on,
    DROP COLUMN ask_how_heard_about_us,
    DROP COLUMN ask_what_stopping_you,
    DROP COLUMN ask_investment_comfort,
    DROP COLUMN ask_urgency_level;
