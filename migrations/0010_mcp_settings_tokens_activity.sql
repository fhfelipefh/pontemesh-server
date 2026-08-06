CREATE TABLE mcp_settings (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    endpoint_path TEXT NOT NULL DEFAULT '/mcp',
    bind_host TEXT NULL,
    require_auth BOOLEAN NOT NULL DEFAULT TRUE,
    read_tools_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    write_tools_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    expose_resources BOOLEAN NOT NULL DEFAULT TRUE,
    expose_prompts BOOLEAN NOT NULL DEFAULT TRUE,
    allow_localhost_only BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT mcp_settings_singleton CHECK (id = TRUE)
);

INSERT INTO mcp_settings (id)
VALUES (TRUE)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE mcp_access_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    token_prefix TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_by_user_id UUID NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ NULL,
    last_used_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_mcp_access_tokens_active ON mcp_access_tokens(is_active, revoked_at);

CREATE TABLE mcp_activity_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_id UUID NULL REFERENCES mcp_access_tokens(id) ON DELETE SET NULL,
    method TEXT NOT NULL,
    target TEXT NULL,
    outcome TEXT NOT NULL,
    detail JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_mcp_activity_events_created_at ON mcp_activity_events(created_at DESC);
CREATE INDEX idx_mcp_activity_events_token_id ON mcp_activity_events(token_id);
