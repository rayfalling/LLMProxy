CREATE TABLE IF NOT EXISTS api_key_model_acl (
    id          TEXT PRIMARY KEY,
    api_key_id  TEXT NOT NULL REFERENCES api_keys(id),
    model_name  TEXT NOT NULL,
    allowed     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(api_key_id, model_name)
);

CREATE INDEX IF NOT EXISTS idx_api_key_model_acl_key ON api_key_model_acl(api_key_id);

CREATE TABLE IF NOT EXISTS tenant_metrics_hourly (
    id                  TEXT PRIMARY KEY,
    tenant_id           TEXT NOT NULL REFERENCES tenants(id),
    hour_bucket         TEXT NOT NULL,
    request_count       INTEGER NOT NULL DEFAULT 0,
    error_count         INTEGER NOT NULL DEFAULT 0,
    failover_count      INTEGER NOT NULL DEFAULT 0,
    total_input_tokens  INTEGER NOT NULL DEFAULT 0,
    total_output_tokens INTEGER NOT NULL DEFAULT 0,
    total_latency_ms    INTEGER NOT NULL DEFAULT 0,
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(tenant_id, hour_bucket)
);

CREATE INDEX IF NOT EXISTS idx_tenant_metrics_hourly_tenant ON tenant_metrics_hourly(tenant_id, hour_bucket);
