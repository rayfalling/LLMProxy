-- 0006_global_username_unique.sql
--
-- Replace the `UNIQUE(tenant_id, username)` constraint on `tenant_admins`
-- with a global `UNIQUE(username)`. This makes login identifiable by
-- username alone, removing the need for the operator to remember and
-- type the tenant name on every sign-in.
--
-- Rollback path (if a future change reintroduces multi-tenant admin UX
-- that legitimately wants the same username under different tenants):
--   1. Drop UNIQUE(username) by rebuilding tenant_admins with the prior
--      composite UNIQUE(tenant_id, username).
--   2. Reintroduce `tenant_name` (or any other disambiguator) on the
--      login API.
--
-- SQLite cannot ALTER an existing UNIQUE constraint in place, so we use
-- the standard table-rebuild recipe. sqlx wraps every migration in its
-- own transaction, so an explicit BEGIN/COMMIT here would error with
-- "cannot start a transaction within a transaction".

CREATE TABLE tenant_admins_new (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (tenant_id) REFERENCES tenants(id)
);

INSERT INTO tenant_admins_new (id, tenant_id, username, password_hash, status, created_at, updated_at)
SELECT id, tenant_id, username, password_hash, status, created_at, updated_at
FROM tenant_admins;

DROP TABLE tenant_admins;
ALTER TABLE tenant_admins_new RENAME TO tenant_admins;

CREATE INDEX IF NOT EXISTS idx_tenant_admins_tenant ON tenant_admins(tenant_id);
-- The UNIQUE(username) constraint provides an implicit index, so an
-- explicit `idx_tenant_admins_username` is no longer needed.
