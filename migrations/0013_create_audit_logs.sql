CREATE TABLE audit_logs (
    unid            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_unid       UUID REFERENCES users(unid) ON DELETE SET NULL,
    action          TEXT NOT NULL,
    resource_type   TEXT NOT NULL,
    resource_unid   UUID,
    metadata        JSONB,
    ip_address      INET,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX audit_logs_created_idx ON audit_logs (created_at DESC);
CREATE INDEX audit_logs_resource_idx ON audit_logs (resource_type, resource_unid);
