-- Plan 45: an MCP client (claude.ai, ChatGPT…) registers itself before a
-- host connects it. Registration is open, as OAuth dynamic client
-- registration is; a client is only a name and where to send the host back.
CREATE TABLE oauth_clients (
    -- The client_id the client sends.
    unid          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_name   TEXT NOT NULL,
    redirect_uris TEXT[] NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT oauth_clients_name CHECK (char_length(client_name) BETWEEN 1 AND 80),
    CONSTRAINT oauth_clients_redirects CHECK (cardinality(redirect_uris) BETWEEN 1 AND 10)
);
