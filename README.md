# LLMProxy

OpenRouter-style LLM reverse proxy with an admin dashboard **and an embedded WebUI**.  
Supports Claude and OpenAI wire protocols, 7 provider adapters, multimodal image routing, per-tenant API-key isolation, fixed-priority failover, per-model outbound proxy, upstream key-pool quota/rate-limiting, and a JWT-protected control-plane REST API.

The dashboard binary embeds a React/Vite SPA, so a single `cargo build --release -p dashboard` produces a self-contained admin UI on port 8081 — no separate web server needed.

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
# Production build (embeds the WebUI into the dashboard binary):
cd web && npm install && npm run build && cd ..
cargo build --release
# binaries at target/release/proxy  and  target/release/dashboard

# Dev workflow (hot-reload UI on :5173 proxying to dashboard on :8081):
cargo run -p dashboard            # backend on 8081
cd web && npm install && npm run dev   # vite dev server on 5173
# Vite proxies /api → :8081 (see web/vite.config.ts).
```

> **Heads-up:** `crates/dashboard/build.rs` automatically creates a
> `web/dist/index.html` placeholder before each `cargo build`, so cargo
> never fails on a fresh clone where the SPA hasn't been bundled yet.
> The placeholder simply tells the user to run `npm run build`. Run
> `npm run build` for any deploy that needs a real UI.

---

## First-Boot Setup

When the dashboard starts against an empty database it has no admin
credentials. Visit **`http://<host>:8081/`** in a browser — `SetupGuard`
detects the empty DB and redirects to **`/setup`**, where the wizard
asks for:

1. Tenant name (e.g. `acme`)
2. Admin username
3. Password (≥ 8 chars) + confirmation

On submit the dashboard hashes the password with **Argon2id** inside a
single SQL transaction that creates both the `tenants` row and the
`tenant_admins` row. Subsequent visits to `/setup` are blocked with HTTP
409 (`already_initialized`); the SPA also auto-redirects already-set-up
browsers back to `/`.

> The setup endpoint is the **only** unauthenticated mutation in the
> control-plane API. Every other `PUT/POST` requires
> `Authorization: Bearer <JWT>` from `POST /api/auth/login`.

After setup completes you can either keep using the WebUI for everything
or pop the demo seed (`scripts/seed.sql`) into the same SQLite file to
get a working OpenAI alias for `curl` smoke-tests — see
[Seeding](#seeding-demo-data) below.

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

## Seeding Demo Data

Production deployments should drive everything through the WebUI
(`/providers`, `/aliases`, `/keys`, `/vision` pages) — but for a quick
`curl` smoke-test, [`scripts/seed.sql`](scripts/seed.sql) inserts a
demo OpenAI provider, two models, an alias, a downstream API key
(`llmproxy-demo-key-replace-me`), and the required key-pool binding.

```bash
# 1. Complete /setup in the WebUI to create the tenant + admin.
# 2. Capture the tenant id:
TENANT_ID=$(sqlite3 llmproxy.db "SELECT id FROM tenants LIMIT 1;")

# 3. Apply the seed (uses sqlite3 named-parameter binding):
sqlite3 llmproxy.db \
  ".param set :tenant_id '$TENANT_ID'" \
  ".read scripts/seed.sql"

# 4. (Optional) Replace the placeholder OpenAI key:
sqlite3 llmproxy.db \
  "UPDATE provider_keys SET key_ref = 'sk-…real…' WHERE id = 'seed-openai-key';"

# 5. Smoke-test through the proxy:
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer llmproxy-demo-key-replace-me" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"hi"}]}'
```

To compute the SHA-256 of a different downstream key:

```bash
scripts/hash-key.sh 'my-real-client-key'        # Linux/macOS
scripts/hash-key.ps1 'my-real-client-key'       # Windows
```

> **Important:** `scripts/seed.sql` does **not** create the tenant or
> admin. Always run the WebUI `/setup` wizard first — it's the only
> path that produces a correctly-hashed Argon2id password.

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
POST /api/setup                                   → first-boot only (no JWT)
GET  /api/setup/status                            → {initialized: bool}
POST /api/auth/login                              → JWT
```

All endpoints below additionally require `Authorization: Bearer <JWT>`:

```ant identity
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
# Rust workspace (28 tests):
cargo test --workspace

# Frontend unit tests (Vitest, 13 tests):
cd web && npm run test:unit

# End-to-end (Playwright, 6 tests, spawns a real dashboard with a temp SQLite):
scripts/e2e.ps1     # Windows
scripts/e2e.sh      # Linux/macOS
```

The Rust tests use `sqlite::memory:` — no file-system state
## Running Tests

```bash
cargo test --workspace
```

Tests use `sqlite::memory:` — no file system state is required.
