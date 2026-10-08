-- Per-host booking form questions, all hidden until the host turns them on.
-- Column names follow the `leads` columns they gate.
ALTER TABLE widget_settings
    ADD COLUMN ask_currently_working_on BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN ask_how_heard_about_us   BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN ask_what_stopping_you    BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN ask_investment_comfort   BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN ask_urgency_level        BOOLEAN NOT NULL DEFAULT false,
    ADD CONSTRAINT widget_public_slug_format CHECK (
        public_slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'
        AND char_length(public_slug) BETWEEN 3 AND 40
    );
