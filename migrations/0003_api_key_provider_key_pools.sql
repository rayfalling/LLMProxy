CREATE TABLE IF NOT EXISTS api_key_provider_key_pools (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL REFERENCES tenants(id),
    api_key_id      TEXT NOT NULL REFERENCES api_keys(id),
    provider_key_id TEXT NOT NULL REFERENCES provider_keys(id),
    created_at      TEXT NOT NULL,
    UNIQUE(api_key_id, provider_key_id)
);

CREATE INDEX IF NOT EXISTS idx_key_pool_tenant ON api_key_provider_key_pools(tenant_id);
CREATE INDEX IF NOT EXISTS idx_key_pool_api_key ON api_key_provider_key_pools(api_key_id);
CREATE INDEX IF NOT EXISTS idx_key_pool_provider_key ON api_key_provider_key_pools(provider_key_id);
