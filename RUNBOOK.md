# Operational Runbook

## 0. Database Wiped / Fresh Host (First-Boot Setup)

### Symptoms
- `sqlite3 llmproxy.db "SELECT count(*) FROM tenants;"` returns `0`.
- `GET /api/setup/status` returns `{"initialized": false}`.
- Every authenticated `/api/*` call returns 401.
- The WebUI auto-redirects every visitor to `/setup`.

### Resolution

1. **Verify both services are up** but the DB really is empty:
   ```bash
   curl -s http://<host>:8081/healthz                         # → "ok"
   curl -s http://<host>:8081/api/setup/status                # → {"initialized":false}
   ```
2. **Open the WebUI** at `http://<host>:8081/` from a trusted operator
   workstation. The SetupGuard will redirect to `/setup` automatically.
3. **Fill the wizard** — tenant name, admin username, password (≥ 8 chars).
   Submit. The dashboard creates the tenant and Argon2id-hashed admin in
   one transaction.
4. **(Optional) Apply demo seed** for an immediate `curl` smoke-test:
   ```bash
   TENANT_ID=$(sqlite3 /var/lib/llmproxy/llmproxy.db \
                 "SELECT id FROM tenants LIMIT 1;")
   sqlite3 /var/lib/llmproxy/llmproxy.db \
     ".param set :tenant_id '$TENANT_ID'" \
     ".read /opt/llmproxy/scripts/seed.sql"
   ```
5. **Re-create your providers / aliases / key-pools** in the WebUI
   (Providers → Aliases → Key Pools → Vision tabs), or by replaying a
   backup of the SQLite file if you keep one.

### "Setup endpoint inaccessible"

If `POST /api/setup` returns 409 `already_initialized` but you still
can't log in, the DB is not actually empty — check
`SELECT username FROM tenant_admins;` and either reset that row's
password by **deleting the entire SQLite file and starting over** (no
in-place reset path is exposed by design — protects against credential
reset bypass) or restore from a known-good backup.

### Prevention

- Snapshot `llmproxy.db` (and its WAL/SHM siblings) on a schedule:
  `sqlite3 llmproxy.db ".backup '/var/backups/llmproxy.db'"`.
- Never delete the DB file as a "restart" technique — it triggers the
  setup wizard on the next visit.

---

## 1. Failover Incident Response

### Symptoms
- Clients receiving 5xx errors
- Dashboard `/api/events/failovers` shows sustained failover activity
- `RUST_LOG=proxy=warn` shows repeated "AllProvidersExhausted" or "UpstreamError"

### Diagnosis

```bash
# 1. Check recent failover events (requires admin JWT)
curl -H "Authorization: Bearer $TOKEN" http://localhost:8081/api/events/failovers?limit=50

# 2. Check provider health state in the DB
sqlite3 llmproxy.db "SELECT id, name, enabled, health_state FROM providers;"

# 3. Check tenant stats
curl -H "Authorization: Bearer $TOKEN" http://localhost:8081/api/stats
```

### Resolution

**Temporarily disable a failed provider:**
```bash
curl -X PUT -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"enabled": false}' \
     http://localhost:8081/api/providers/<provider-id>/enabled
```
The proxy picks up the change on the next request (no restart needed).

**Re-enable after the provider recovers:**
```bash
curl -X PUT -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"enabled": true}' \
     http://localhost:8081/api/providers/<provider-id>/enabled
```

**Rotate to a backup alias target:**
```bash
# List current targets
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8081/api/aliases/<alias-name>/targets

# Replace targets (lower priority number = tried first)
curl -X PUT -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"targets": [
           {"provider_id": "backup-provider", "model_name": "gpt-4o", "priority": 0, "enabled": true},
           {"provider_id": "primary-provider", "model_name": "gpt-4o", "priority": 1, "enabled": false}
         ]}' \
     http://localhost:8081/api/aliases/<alias-name>/targets
```

---

## 2. Upstream Key Rotation

When a provider API key is revoked or rotated:

```sql
-- Update key in DB (proxy reads it on each request, no restart needed)
UPDATE provider_keys
SET key_ref = 'sk-new-key-value'
WHERE id = '<provider-key-id>';
```

To add a key to a downstream API key's pool:
```bash
curl -X PUT -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"provider_key_ids": ["pk-id-1", "pk-id-2"]}' \
     http://localhost:8081/api/key-pools/<api-key-id>
```

---

## 3. Per-Region Outbound Proxy Issues

If a provider is only reachable via a specific outbound proxy and the proxy fails:

```bash
# 1. Check current model-level proxy assignments
sqlite3 llmproxy.db \
  "SELECT pm.model_name, op.name, op.host, op.port
   FROM provider_models pm
   LEFT JOIN outbound_proxies op ON op.id = pm.outbound_proxy_id;"

# 2. Disable the faulty outbound proxy
sqlite3 llmproxy.db \
  "UPDATE outbound_proxies SET enabled = 0 WHERE id = '<proxy-id>';"

# 3. Assign a different proxy to affected models
sqlite3 llmproxy.db \
  "UPDATE provider_models SET outbound_proxy_id = '<backup-proxy-id>'
   WHERE provider_id = '<provider-id>';"
```

The proxy binary reads `outbound_proxy_id` on startup when bootstrapping the `FailoverEngine`.  
**A restart is required** after changing `outbound_proxies` or `outbound_proxy_id` assignments.

---

## 4. Database Maintenance

### WAL checkpoint (run periodically to keep the WAL file small)
```bash
sqlite3 llmproxy.db "PRAGMA wal_checkpoint(TRUNCATE);"
```

### Prune old request logs (keep 30 days)
```bash
sqlite3 llmproxy.db \
  "DELETE FROM request_logs WHERE created_at < datetime('now', '-30 days');"
```

### Prune old hourly metrics (keep 90 days)
```bash
sqlite3 llmproxy.db \
  "DELETE FROM tenant_metrics_hourly WHERE hour_bucket < datetime('now', '-90 days');"
```

---

## 5. Restart Procedure

Both binaries are stateless relative to each other (all state is in SQLite).

```bash
# Docker Compose
docker compose restart proxy
docker compose restart dashboard

# systemd
systemctl restart llmproxy-proxy
systemctl restart llmproxy-dashboard
```

The proxy reloads all runtime state (providers, keys, aliases, vision mappings) from the DB on startup.

---

## 6. Quota / Rate-Limit Alerts

If clients are hitting `QuotaExceeded` or `RateLimitExceeded`:

```sql
-- Check current daily quota usage for a tenant's key
SELECT ak.name, ak.quota_daily_req,
       COUNT(*) AS used_today
FROM api_keys ak
LEFT JOIN request_logs rl
  ON rl.api_key_id = ak.id
  AND rl.created_at >= date('now')
WHERE ak.tenant_id = '<tenant-id>'
GROUP BY ak.id;
```

To raise the daily limit:
```sql
UPDATE api_keys SET quota_daily_req = 10000 WHERE id = '<api-key-id>';
```

---

## 7. Health Checks

| Endpoint | Expected response |
|---|---|
| `GET http://localhost:8080/healthz` | `200 ok` |
| `GET http://localhost:8081/healthz` | `200 ok` |

Both respond immediately without DB access — suitable for load-balancer probes.
