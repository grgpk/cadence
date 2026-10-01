CREATE TABLE roleaccesses (
    unid            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    grantedto_unid  UUID NOT NULL REFERENCES users(unid) ON DELETE CASCADE,
    role            TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT roleaccesses_role CHECK (role IN ('Root', 'Admin', 'Host')),
    CONSTRAINT roleaccesses_user_role_unique UNIQUE (grantedto_unid, role)
);

CREATE INDEX roleaccesses_user_idx ON roleaccesses (grantedto_unid);
