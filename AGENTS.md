# AGENTS.md — LLMProxy contributor guide

This file is the canonical source of repo conventions for any AI agent or
human contributor working in `LLMProxy`. It is mirrored verbatim by
[`CLAUDE.md`](CLAUDE.md). Update both together.

---

## 1. What this repo is

A Rust workspace + React WebUI implementing a **multi-tenant LLM proxy**:

- `crates/proxy` — Axum HTTP server on `:8080` translating between the
  Claude / OpenAI wire formats and 7 upstream provider adapters, with
  fixed-priority failover, per-model outbound proxy, and per-key quota
  enforcement.
- `crates/dashboard` — Axum control-plane on `:8081` (REST + JWT) that
  also serves the embedded React SPA from `web/dist/`.
- `crates/llm-core` — shared types, DB pool, and migration runner.
- `migrations/` — sqlx migrations against a single SQLite file shared
  by both binaries.
- `web/` — Vite + React 18 + TypeScript + Tailwind SPA. Built into
  `web/dist/` and embedded into the dashboard binary at compile time
  via `include_dir!`.

Both binaries point at the **same** `DATABASE_URL` (default
`sqlite://llmproxy.db`).

## 2. Architectural invariants — DO NOT BREAK

1. **Tenancy boundaries**. Every authenticated dashboard handler MUST
   derive `tenant_id` from the `TenantAdmin` extractor (JWT claims),
   never from the request body or path. Cross-tenant access returns
   `404 Not Found`, never `403`, to avoid leaking existence.
2. **Resource scope**:
   - **Global, admin-only**: `providers`, `provider_keys`,
     `provider_models`, `model_aliases`, `model_alias_targets`,
     `failover_rules`, `model_vision_mappings`, `outbound_proxies`.
   - **Tenant-scoped**: `api_keys`, `api_key_provider_key_pools`,
     `api_key_model_acl`, `tenant_metrics_hourly`, `request_logs`.
3. **Login is tenantless** (since migration `0006`). Username is
   globally unique. Tenant is captured only at first-boot setup and
   resolved automatically at login.
4. **Setup is single-shot**. `POST /api/setup` is the only
   unauthenticated mutation; subsequent calls return `409
   already_initialized`. There is **no in-place admin password reset**
   exposed by the API — recovery is "delete the SQLite file and
   re-bootstrap" by design.
5. **API-key plaintext is shown once**. `POST /api/api-keys` returns
   `plaintext_key` exactly once; `GET /api/api-keys` only ever exposes
   the prefix. Plaintext is stored verbatim in `api_keys.hashed_key`
   (the column name is historical — proxy `auth.rs` compares the raw
   bearer token against this column, no hashing). Never log it.
6. **No content-based proxy auth bypass**. The proxy `/v1/*` endpoints
   require `Authorization: Bearer lp_…`; the dashboard `auth_layer`
   wraps every `/api/*` route except `/api/setup`, `/api/setup/status`,
   `/api/auth/login`, and `/healthz`.

## 3. Build & test commands

```pwsh
# Rust (workspace)
$env:PATH = "C:\Users\wanghaiwei\.cargo\bin;$env:PATH"
cargo build -p dashboard -p proxy
cargo test  --workspace                       # all backend tests
cargo test -p dashboard --test integration    # auth + dashboard handlers
cargo test -p dashboard --test crud           # 11 CRUD + tenant-isolation cases

# Frontend
cd web
npm ci
npm run build         # tsc + vite (this also produces web/dist/ which the
                      # dashboard binary embeds at compile time)
npm run test:unit     # vitest
npm run test:e2e      # playwright (requires a running dashboard)
```

Backend test count baseline: **41 passing** (10 integration + 11 crud +
7 setup + 3 static_fallback + 10 routing/failover + others). Frontend
unit baseline: **18 passing**. Drop these baselines only with a
matching commit explanation.

Use `;` to chain PowerShell commands. **Never use `&&`** in PowerShell —
it is not a valid statement separator in Windows PowerShell 5.1.

## 4. Code-style rules

### Rust

- One handler module: `crates/dashboard/src/handlers.rs`. Add new
  handlers there in the same style: `async fn`, return
  `Result<Json<…>, ApiError>`, derive `tenant_id` from the
  `TenantAdmin` extractor.
- Validation helpers: reuse `bad_request(error, message, field)`,
  `conflict(...)`, `not_found(...)`, `valid_provider_name(...)`,
  `mask_secret(...)`, `generate_api_key()`. Do not duplicate.
- Multi-row inserts (alias + targets, etc.) must run inside a
  `pool.begin()` transaction.
- Routes are wired in `crates/dashboard/src/main.rs` using axum method
  chaining: `.route("/api/x/{id}", get(list).post(create))`.
- Migrations: **no explicit `BEGIN`/`COMMIT`** — sqlx 0.8 already
  wraps each migration in a transaction. Use the SQLite
  table-rebuild recipe (`CREATE TABLE _new` → `INSERT…SELECT` → `DROP`
  → `ALTER…RENAME`) for any UNIQUE/PK change.
- Avoid `.unwrap()` in non-test code. Prefer `?` with a typed error.

### Frontend

- All API calls go through `web/src/api/client.ts`; types live in
  `web/src/api/types.ts`. Do not add `axios` calls in pages.
- Pages use the shared modal kit:
  - [`ResourceCreateModal`](web/src/components/ResourceCreateModal.tsx)
    for "+ Add X" forms.
  - [`ConfirmDeleteDialog`](web/src/components/ConfirmDeleteDialog.tsx)
    for destructive actions; pass `typedConfirmation={resource.name}`
    for high-blast-radius deletes (providers, aliases, etc.).
  - [`RevealOnceModal`](web/src/components/RevealOnceModal.tsx) for any
    one-time secret display (currently API keys only).
- Page state pattern:
  ```ts
  type Modal =
    | { kind: 'add-x' }
    | { kind: 'delete-x'; target: X }
  const [modal, setModal] = useState<Modal | null>(null)
  ```
  Modal selectors live at the bottom of the JSX. Always `await load()`
  (or a scoped refresher) after a successful mutation.
- Surface server errors via `err.response?.data?.message` into local
  state — never `alert()`.
- Pure helpers (no DOM, no JSX) belong in `web/src/api/*-helpers.ts`
  with matching `*-helpers.test.ts` files. See
  [`webui-helpers.ts`](web/src/api/webui-helpers.ts) for the pattern.
- Pre-existing `enabled` columns are returned as `0|1` from the backend
  (sqlite booleans) — type them as `number`, not `boolean`.

## 5. OpenSpec workflow

This repo uses [OpenSpec](https://openspec.dev) for spec-driven
development. The flow:

1. **Propose** a change: `openspec change add <slug>` produces a
   directory under `openspec/changes/<slug>/` with `proposal.md`,
   `design.md`, `tasks.md`, and a `specs/` delta.
2. **Validate strictly**: `openspec validate <slug> --strict` must be
   clean before any code lands.
3. **Implement** by ticking tasks in `tasks.md`. Tests + docs are
   first-class tasks, not afterthoughts.
4. **Archive** when done: `openspec archive <slug>` (use
   `--skip-specs` when the change deliberately defers spec updates).

Typical tasks.md outline (8 sections, all required):

```
1. Schema / API surface          5. Frontend management pages
2. Backend handlers              6. Frontend tests
3. Backend tests                 7. Docs (README + RUNBOOK)
4. Frontend shared components    8. Validate + bare-metal + archive
```

Section 8 is **not** optional — it includes (a) `cargo test`, (b)
`npm run test:unit`, (c) deploy + smoke-test on the bare-metal LXC at
`192.168.50.64`, (d) strict validate, (e) archive.

## 6. Bare-metal smoke-test target (192.168.50.64)

- Layout: source under `/opt/llmproxy/src`, binaries in
  `/opt/llmproxy/bin/`, SQLite DB in `/opt/llmproxy/data/llmproxy.db`,
  systemd units `llmproxy-proxy.service` and
  `llmproxy-dashboard.service`.
- Deploy recipe (PowerShell):
  ```pwsh
  git archive --format=tar.gz -o ../llmproxy-src.tar.gz HEAD
  scp ../llmproxy-src.tar.gz root@192.168.50.64:/tmp/
  ssh root@192.168.50.64 "cd /opt/llmproxy/src && \
      rm -rf crates migrations web scripts openspec README.md RUNBOOK.md \
             Cargo.toml Cargo.lock docker-compose.yml Dockerfile && \
      tar -xzf /tmp/llmproxy-src.tar.gz && \
      cargo build --release -p dashboard -p proxy && \
      cd web && npm ci --silent && npm run build && cd .. && \
      cargo build --release -p dashboard && \
      systemctl stop llmproxy-dashboard llmproxy-proxy && \
      cp target/release/dashboard /opt/llmproxy/bin/ && \
      cp target/release/proxy     /opt/llmproxy/bin/ && \
      systemctl start llmproxy-proxy llmproxy-dashboard"
  ```
  Note the **double rebuild** of `dashboard`: the SPA must be built
  before the second cargo build so `include_dir!("../../web/dist")`
  picks up the fresh assets.
- For DB-altering changes, wipe `/opt/llmproxy/data/llmproxy.db*` first.
- JSON over SSH: build the body locally, base64-encode, decode on the
  far side. Inline `'{"…"}'` quoting through PowerShell → bash always
  goes wrong.

## 7. PR / commit conventions

- Conventional Commits with scope: `feat(dashboard): …`,
  `test(web): …`, `fix(migration): …`, `docs: …`,
  `chore(openspec): …`.
- Each section of an OpenSpec change typically lands as a separate
  commit so the history doubles as a progress log.
- Don't `git push --force` on `main`. Don't bypass commit hooks.

## 8. Things that have bitten us before

- `cannot start a transaction within a transaction` from sqlx 0.8 →
  remove explicit `BEGIN`/`COMMIT` from migrations.
- Frontend `enabled: boolean` → backend rejects with TS2322 because
  the row literal is `number`. Use `1` / `0`.
- PowerShell stderr redirects: `git push` writes progress to stderr,
  PowerShell flags it as "RemoteException". Exit code is what matters.
- `replace_string_in_file` with massive blocks has corrupted
  `handlers.rs` before by truncating to null bytes. For surgery on
  long files, anchor on a small unique helper signature and replace a
  short region at a time.
- `create_file` refuses to overwrite. To re-create a file, delete it
  first (`Remove-Item path`).

## 9. Where to look first

| Question | Start in |
|---|---|
| How does login work? | [crates/dashboard/src/auth.rs](crates/dashboard/src/auth.rs) |
| How is a request routed to upstreams? | [crates/proxy/src/routing/](crates/proxy/src/routing/) |
| What endpoints exist on the dashboard? | [crates/dashboard/src/main.rs](crates/dashboard/src/main.rs) |
| What does the SPA render? | [web/src/App.tsx](web/src/App.tsx), [web/src/components/AppLayout.tsx](web/src/components/AppLayout.tsx) |
| Schema | [migrations/](migrations/) |
| Operational playbook | [RUNBOOK.md](RUNBOOK.md) |
| Live spec docs | `openspec list`, `openspec list --specs` |
