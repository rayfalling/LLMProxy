# LLMProxy

OpenRouter-style LLM reverse proxy with an admin dashboard.  
Supports Claude and OpenAI wire protocols, 7 provider adapters, multimodal image routing, per-tenant API-key isolation, fixed-priority failover, per-model outbound proxy, upstream key-pool quota/rate-limiting, and a JWT-protected control-plane REST API.

## Architecture

```
Client
  │ POST /v1/chat/completions  (OpenAI protocol)
  │ POST /v1/messages          (Claude protocol)
  ▼
┌─────────────────────────────────────────────────────────┐
│  proxy  (port 8080)                                      │
│  ┌─────────────────────────┐  ┌─────────────────────┐   │
│  │ Protocol inbound layer  │  │ Auth / quota check  │   │
│  │  (OpenAI ↔ Canonical)   │  │ (api_keys table)    │   │
│  └────────────┬────────────┘  └─────────────────────┘   │
│               │ CanonicalRequest                          │
│  ┌────────────▼────────────────────────────────────────┐ │
│  │  MultimodalRouter                                    │ │
│  │  ┌───────────────────┐  ┌──────────────────────┐    │ │
│  │  │ same-model vision │  │ split pipeline        │    │ │
│  │  │ (supports_vision) │  │ parser → strip → gen  │    │ │
│  │  └───────────────────┘  └──────────────────────┘    │ │
│  └────────────┬────────────────────────────────────────┘ │
│               │                                           │
│  ┌────────────▼────────────────────────────────────────┐ │
│  │  FailoverEngine (fixed-priority)                     │ │
│  │  AliasRegistry → [target₁, target₂, …]              │ │
│  │  pick_key (key-pool ACL) → build ExecContext         │ │
│  └────────────┬────────────────────────────────────────┘ │
│               │ ProviderAdapter::complete()               │
│     Anthropic / OpenAI / Copilot / Xiaomi /              │
│     Google / OpenRouter / DeepSeek                       │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│  dashboard  (port 8081)  — JWT-protected REST API        │
│  Manage: providers, models, aliases, key-pools,          │
│          vision mappings, failover events, stats         │
└─────────────────────────────────────────────────────────┘
```

---

## Quick Start

### Docker Compose

```bash
# 1. Copy and edit secrets
cp .env.example .env
# set JWT_SECRET to a random 32+ char string

# 2. Start
JWT_SECRET=your-secret docker compose up -d

# 3. Seed initial tenant and admin (see "Seeding" below)
```

### Build from source

```bash
cargo build --release
# binaries at target/release/proxy  and  target/release/dashboard
```

---

## Environment Variables

### proxy binary

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite://llmproxy.db` | SQLite file path (supports `sqlite::memory:` for tests) |
| `PROXY_HOST` | `0.0.0.0` | Bind address |
| `PROXY_PORT` | `8080` | HTTP listen port |
| `RUST_LOG` | `proxy=info,tower_http=info` | Log filter (tracing) |

### dashboard binary

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite://llmproxy.db` | Must point to the **same** SQLite file as proxy |
| `DASHBOARD_HOST` | `0.0.0.0` | Bind address |
| `DASHBOARD_PORT` | `8081` | HTTP listen port |
| `JWT_SECRET` | `change-me-in-production` | HMAC-HS256 signing key — **must** be overridden |
| `JWT_EXPIRY_SECS` | `86400` | Token lifetime in seconds |
| `RUST_LOG` | `dashboard=info,tower_http=info` | Log filter |

---

## Database

Both binaries share one **SQLite** file.  
On startup each binary calls `connect_and_migrate()` which applies all pending migrations from the `migrations/` directory.

SQLite is opened with:
- `PRAGMA journal_mode = WAL` — concurrent readers
- `PRAGMA foreign_keys = ON` — referential integrity
- Pool max 16 connections

### Schema (5 migrations)

| Migration | Tables added |
|---|---|
| 0001_initial | tenants, api_keys, providers, provider_keys, provider_models, outbound_proxies, model_aliases, model_alias_targets, failover_rules, request_logs |
| 0002_tenant_admins | tenant_admins |
| 0003_api_key_provider_key_pools | api_key_provider_key_pools |
| 0004_security_observability | api_key_model_acl, tenant_metrics_hourly |
| 0005_model_vision_mappings | model_vision_mappings |

---

## Seeding Initial Data

There is no web UI for initial setup — seed via SQL or a script:

```sql
-- 1. Create a tenant
INSERT INTO tenants (id, name, status, created_at, updated_at)
VALUES ('your-uuid', 'my-org', 'active', datetime('now'), datetime('now'));

-- 2. Hash a password with argon2id (use the argon2 CLI or a script):
--    argon2 <salt> -id -t 3 -m 12 -p 1 -l 32 -e
INSERT INTO tenant_admins (id, tenant_id, username, password_hash, status)
VALUES ('admin-uuid', 'your-uuid', 'admin', '$argon2id$...', 'active');

-- 3. Register an upstream provider
INSERT INTO providers (id, name, display_name, base_url, enabled, health_state, created_at, updated_at)
VALUES ('openai-1', 'openai', 'OpenAI', 'https://api.openai.com', 1, 'unknown', datetime('now'), datetime('now'));

-- 4. Add an API key for that provider
INSERT INTO provider_keys (id, provider_id, key_ref, enabled, priority, created_at)
VALUES ('pk-1', 'openai-1', 'sk-...your-key...', 1, 0, datetime('now'));

-- 5. Register a model
INSERT INTO provider_models (id, provider_id, model_name, enabled, supports_vision, created_at, updated_at)
VALUES ('gpt4o-1', 'openai-1', 'gpt-4o', 1, 1, datetime('now'), datetime('now'));

-- 6. Create a logical alias
INSERT INTO model_aliases (id, alias_name, route_strategy, created_at, updated_at)
VALUES ('alias-1', 'gpt-4o', 'priority', datetime('now'), datetime('now'));

-- 7. Map alias to provider model
INSERT INTO model_alias_targets (id, alias_id, provider_id, model_name, priority, enabled, created_at)
VALUES ('t-1', 'alias-1', 'openai-1', 'gpt-4o', 0, 1, datetime('now'));

-- 8. Create a downstream API key for clients
INSERT INTO api_keys (id, tenant_id, hashed_key, name, status, created_at, updated_at)
VALUES ('ak-1', 'your-uuid', 'sha256-hash-of-client-key', 'client-key-1', 'active', datetime('now'), datetime('now'));
```

The `hashed_key` field stores **SHA-256** of the raw key (the proxy hashes the bearer token on every request).

---

## API Reference

### Proxy endpoints

| Method | Path | Protocol |
|---|---|---|
| `POST` | `/v1/chat/completions` | OpenAI ChatCompletion |
| `POST` | `/v1/messages` | Anthropic Messages |
| `GET`  | `/healthz` | Health check |

Authenticate with `Authorization: Bearer <your-downstream-api-key>`.

### Dashboard endpoints (all require `Authorization: Bearer <JWT>`)

```
POST /api/auth/login                              → JWT
GET  /api/me                                      → tenant identity
GET  /api/providers                               → list providers
PUT  /api/providers/{id}/enabled                  → enable/disable provider
GET  /api/providers/{id}/models                   → list models
PUT  /api/providers/{id}/models/{name}/enabled    → enable/disable model
GET  /api/aliases                                 → list aliases
PUT  /api/aliases/{name}/strategy                 → update route strategy
PUT  /api/aliases/{name}/targets                  → replace alias targets
GET  /api/key-pools                               → list key-pool mappings
PUT  /api/key-pools/{api_key_id}                  → update key-pool mapping
GET  /api/vision-mappings                         → list vision config
PUT  /api/vision-mappings/{model_name}            → update vision config
GET  /api/events/failovers                        → recent failover events
GET  /api/stats                                   → tenant metrics (QPS, p95, error rate)
```

---

## Provider Adapters

| Provider | id | Notes |
|---|---|---|
| Anthropic | `anthropic` | Messages API, supports beta headers |
| OpenAI | `openai` | Chat Completions |
| GitHub Copilot / Azure OpenAI | `copilot` | Bearer + custom base_url |
| Xiaomi | `xiaomi` | OpenAI-compatible |
| Google Gemini | `google` | OpenAI-compatible endpoint |
| OpenRouter | `openrouter` | Adds HTTP-Referer/X-Title headers |
| DeepSeek | `deepseek` | OpenAI-compatible |

Set `base_url` in the `providers` table to override the default endpoint.

---

## Multimodal Image Routing

Two modes (configured per model in `model_vision_mappings`):

1. **Same-model vision** — `supports_vision = 1`: image is forwarded directly to the provider.
2. **Split pipeline** — `supports_vision = 0` with a `vision_parser_alias`:  
   a. Image is sent to the vision parser model to generate a text description.  
   b. The description replaces the image in the original request.  
   c. The text-only request is forwarded to the generation model.

---

## Failover

- Targets per alias are sorted by `priority` (ascending = higher priority).
- When a provider returns a failure that matches any of the alias's `failover_triggers`, the engine retries the next target.
- Trigger values: `insufficient_balance`, `rate_limited`, `server_error`, `timeout`, `model_offline`, `manually_disabled`.
- Empty `failover_triggers` list = failover on all errors.
- Providers can be manually disabled via the dashboard or `PUT /api/providers/{id}/enabled`.

---

## Running Tests

```bash
cargo test --workspace
```

Tests use `sqlite::memory:` — no file system state is required.
