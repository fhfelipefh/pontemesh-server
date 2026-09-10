ALTER TABLE mcp_settings
    ADD COLUMN IF NOT EXISTS auth_mode TEXT NOT NULL DEFAULT 'hybrid';

CREATE TABLE IF NOT EXISTS mcp_oauth_clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id TEXT NOT NULL UNIQUE,
    client_secret_hash TEXT NOT NULL,
    client_name TEXT NOT NULL,
    redirect_uris TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    scopes TEXT[] NOT NULL DEFAULT ARRAY['read']::TEXT[],
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_mcp_oauth_clients_client_id ON mcp_oauth_clients(client_id);

CREATE TABLE IF NOT EXISTS mcp_oauth_codes (
    code TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scopes TEXT[] NOT NULL,
    code_challenge TEXT NULL,
    code_challenge_method TEXT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_mcp_oauth_codes_expires_at ON mcp_oauth_codes(expires_at);

CREATE TABLE IF NOT EXISTS mcp_oauth_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    refresh_token_hash TEXT NULL UNIQUE,
    scopes TEXT[] NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_mcp_oauth_tokens_token_hash ON mcp_oauth_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_mcp_oauth_tokens_refresh_token_hash ON mcp_oauth_tokens(refresh_token_hash);
