CREATE TABLE IF NOT EXISTS tenant_admins (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(tenant_id, username),
    FOREIGN KEY (tenant_id) REFERENCES tenants(id)
);

CREATE INDEX IF NOT EXISTS idx_tenant_admins_tenant ON tenant_admins(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_admins_username ON tenant_admins(username);
