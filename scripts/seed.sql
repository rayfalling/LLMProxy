-- LLMProxy demo seed data.
--
-- IMPORTANT: this script does NOT create the first tenant or admin
-- account. Use the WebUI at http://<host>:8081/setup on first boot to
-- bootstrap those (the wizard hashes the admin password with Argon2id
-- inside the dashboard process).
--
-- After /setup completes, capture the tenant id:
--
--   sqlite3 llmproxy.db "SELECT id FROM tenants LIMIT 1;"
--
-- then run this script with `:tenant_id` substituted in. With the
-- sqlite3 CLI:
--
--   sqlite3 llmproxy.db ".param set :tenant_id 'PASTE-UUID-HERE'" \
--                       ".read scripts/seed.sql"
--
-- Or pre-substitute via sed/PowerShell -replace before piping.

BEGIN;

-- 1. An upstream provider (OpenAI). The placeholder API key is stored
--    plain in `provider_keys.key_ref` exactly the way the dashboard's
--    "Add provider key" flow will write it; replace with a real key
--    before running real traffic through this provider.
INSERT OR IGNORE INTO providers
    (id, name, display_name, base_url, enabled, health_state, created_at, updated_at)
VALUES
    ('seed-openai', 'openai', 'OpenAI (demo)', 'https://api.openai.com',
     1, 'unknown', datetime('now'), datetime('now'));

INSERT OR IGNORE INTO provider_keys
    (id, provider_id, key_ref, enabled, priority, created_at)
VALUES
    ('seed-openai-key', 'seed-openai', 'sk-REPLACE-ME', 1, 0, datetime('now'));

INSERT OR IGNORE INTO provider_models
    (id, provider_id, model_name, enabled, supports_vision,
     supports_streaming, supports_tools, created_at, updated_at)
VALUES
    ('seed-gpt4o',     'seed-openai', 'gpt-4o',      1, 1, 1, 1,
     datetime('now'), datetime('now')),
    ('seed-gpt4omini', 'seed-openai', 'gpt-4o-mini', 1, 1, 1, 1,
     datetime('now'), datetime('now'));

-- 2. A logical alias clients will request, mapped to the seeded model
--    with priority 0 (= tried first by the failover engine).
INSERT OR IGNORE INTO model_aliases
    (id, alias_name, description, route_strategy, created_at, updated_at)
VALUES
    ('seed-alias-gpt4o', 'gpt-4o',
     'Demo alias routing to OpenAI gpt-4o (seed data)',
     'priority', datetime('now'), datetime('now'));

INSERT OR IGNORE INTO model_alias_targets
    (id, alias_id, provider_id, model_name, priority, enabled, created_at)
VALUES
    ('seed-target-gpt4o', 'seed-alias-gpt4o', 'seed-openai', 'gpt-4o',
     0, 1, datetime('now')),
    ('seed-target-gpt4omini', 'seed-alias-gpt4o', 'seed-openai',
     'gpt-4o-mini', 1, 1, datetime('now'));

-- 3. A downstream API key clients will present as `Authorization: Bearer …`.
--    The dashboard normally creates these; we precompute the SHA-256
--    hash here so a developer can curl the proxy immediately.
--
--    Plain key:        llmproxy-demo-key-replace-me
--    SHA-256 (hex):    7bf29e0af8ab63e2482049365b03fcc165ffd280c1c7051c479aa1d639c1dd85
--    (See scripts/hash-key.ps1 / hash-key.sh to recompute.)
INSERT OR IGNORE INTO api_keys
    (id, tenant_id, hashed_key, name, status, created_at, updated_at)
VALUES
    ('seed-api-key',
     :tenant_id,
     '7bf29e0af8ab63e2482049365b03fcc165ffd280c1c7051c479aa1d639c1dd85',
     'demo-client-key', 'active',
     datetime('now'), datetime('now'));

-- 4. Bind the demo API key to the demo provider key (required by the
--    key-pool ACL — without this, the proxy will refuse to forward).
INSERT OR IGNORE INTO api_key_provider_key_pools
    (id, tenant_id, api_key_id, provider_key_id, created_at)
VALUES
    ('seed-pool-1', :tenant_id, 'seed-api-key', 'seed-openai-key',
     datetime('now'));

COMMIT;
