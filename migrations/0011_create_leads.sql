-- One visit's booking form, saved as it is filled in. The unid comes from
-- the browser so every save lands on the same row.
CREATE TABLE leads (
    unid                UUID PRIMARY KEY,
    host_unid           UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    contact_unid        UUID REFERENCES contacts(unid) ON DELETE SET NULL,
    -- Where the form was opened from: page and UTM tags live on the visit.
    calling_visit_unid  UUID REFERENCES calling_visits(unid) ON DELETE SET NULL,
    form_submitted_at   TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX leads_host_updated_idx ON leads (host_unid, updated_at DESC);
CREATE INDEX leads_submitted_idx ON leads (form_submitted_at);
CREATE INDEX leads_contact_idx ON leads (contact_unid);

-- An answer to one of the type's questions. Only asked questions keep one.
CREATE TABLE lead_answers (
    lead_unid   UUID NOT NULL REFERENCES leads(unid) ON DELETE CASCADE,
    question    TEXT NOT NULL,
    answer      TEXT NOT NULL,
    PRIMARY KEY (lead_unid, question),
    CONSTRAINT lead_answers_question CHECK (question IN (
        'currently_working_on', 'how_heard_about_us', 'what_stopping_you',
        'investment_comfort', 'urgency_level'
    ))
);
