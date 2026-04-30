## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        Browser (SPA)                             │
│  React Router routes:                                            │
│    /            → Login.tsx                                      │
│    /setup       → Setup.tsx (only reachable when DB empty)       │
│    /dashboard   → Dashboard.tsx (protected)                      │
│    /providers   → ProviderMgmt.tsx (protected)                   │
│    /aliases     → AliasMgmt.tsx (protected)                      │
│    /keys        → KeyPoolMgmt.tsx (protected)                    │
│    /vision      → VisionMgmt.tsx (protected)                     │
└────────────────────────────┬─────────────────────────────────────┘
                             │ HTTP/JSON  (Authorization: Bearer ...)
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│              Dashboard Binary (axum, port 8081)                  │
│                                                                  │
│  Route layers:                                                   │
│    POST /api/setup            (when DB empty; 409 otherwise)     │
│    POST /api/auth/login                                          │
│    GET  /api/me               (JWT required)                     │
│    GET  /api/providers ...    (tenant-scoped, JWT required)      │
│    GET  /api/aliases   ...                                       │
│    GET  /api/stats/...                                           │
│                                                                  │
│  Static fallback:                                                │
│    /, /assets/*, /setup, /dashboard, ...  → embedded SPA         │
│        served from include_dir!("../../web/dist")                │
│        Unknown non-/api paths return index.html (SPA routing).   │
└────────────────────────────┬─────────────────────────────────────┘
                             │
                             ▼
                       SQLite (WAL)
```

## Repository Layout

```
LLMProxy/
├── crates/
│   ├── llm-core/
│   ├── proxy/
│   └── dashboard/         # adds /api/setup, embeds web/dist
├── web/                   # NEW — frontend project (NOT a Cargo crate)
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── tailwind.config.js
│   ├── index.html
│   ├── src/
│   │   ├── main.tsx
│   │   ├── App.tsx
│   │   ├── api/{client.ts,types.ts}
│   │   ├── components/{Header,ProtectedRoute,...}.tsx
│   │   └── pages/{Login,Setup,Dashboard,ProviderMgmt,AliasMgmt,KeyPoolMgmt,VisionMgmt}.tsx
│   ├── tests/             # Playwright specs
│   └── dist/              # Vite build output (gitignored)
└── scripts/
    └── seed.sql           # optional demo seed
```

## Backend Changes (`crates/dashboard`)

### 1. New endpoint `POST /api/setup`

Request:
```json
{ "tenant_name": "...", "username": "...", "password": "...", "password_confirm": "..." }
```

Algorithm:
1. Validate (non-empty, password length ≥ 8, confirm matches).
2. `SELECT COUNT(*) FROM tenants`; if > 0 → return 409
   `{"error":"already_initialized","message":"..."}`.
3. Hash password with Argon2id (reuse existing helper).
4. Insert `tenants` row + `tenant_admins` row in a single transaction.
5. Respond `200 {"success":true}`.

### 2. Empty-DB awareness for SPA routing

Add a small helper `dashboard::bootstrap::is_initialized(pool)` returning
`bool`. The SPA fallback handler:

- If request path begins with `/api/`: pass through to API router.
- Else if uninitialized AND request path is not `/setup` (or `/assets/*`):
  serve `index.html`. The SPA itself, on mount, calls `GET /api/setup/status`
  (a lightweight `{"initialized":bool}` endpoint) and redirects to `/setup`
  when `false`.
- Else: serve embedded SPA assets via `include_dir!`.

This avoids hard-coding redirect logic on the server while still giving the
SPA enough info to drive the wizard.

### 3. Static asset embedding

```rust
// crates/dashboard/src/static_assets.rs
use include_dir::{include_dir, Dir};
pub static WEB_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../web/dist");
```

A `build.rs` ensures `web/dist/index.html` exists at compile time (creates a
1-line placeholder if missing) so cargo builds work even before `npm run
build`. Production deploys are expected to run `npm run build` first.

### 4. Crate dependency additions

`crates/dashboard/Cargo.toml`:
```toml
include_dir = "0.7"
mime_guess  = "2"
```

## Frontend (`web/`)

### Stack

- React 18, TypeScript 5, Vite 5
- Tailwind CSS 3, PostCSS, autoprefixer
- React Router 6, Axios 1
- Playwright 1.40 for E2E, Vitest for unit

### Auth state

- JWT in `localStorage["jwt_token"]`.
- Axios request interceptor injects `Authorization: Bearer <token>`.
- Response interceptor: 401 → clear token + redirect to `/`.

### Setup state

`App.tsx` on first mount calls `GET /api/setup/status`:
- `{"initialized":false}` → if current path ≠ `/setup`, redirect.
- `{"initialized":true}`  → if current path = `/setup`, redirect to `/`.

### Pages

| Page          | API endpoints used                                |
| ------------- | ------------------------------------------------- |
| Login         | `POST /api/auth/login`                            |
| Setup         | `GET /api/setup/status`, `POST /api/setup`        |
| Dashboard     | `GET /api/stats/tenant`, `/api/stats/failover-events` |
| ProviderMgmt  | `GET/PATCH /api/providers`, `/api/providers/:id/models/:mid` |
| AliasMgmt     | `GET/PATCH /api/aliases`                          |
| KeyPoolMgmt   | `GET/PATCH /api/api-keys`, `/api/provider-keys`   |
| VisionMgmt    | `GET/PATCH /api/vision-mappings`                  |

### Dev workflow

```bash
# Terminal A — Rust dashboard
cargo run -p dashboard

# Terminal B — Vite dev server
cd web && npm install && npm run dev
# → http://localhost:5173 (proxies /api → :8081)
```

### Production build

```bash
cd web && npm install && npm run build   # outputs web/dist/
cargo build -p dashboard --release       # embeds web/dist via include_dir!
```

## Testing Strategy

1. **Backend unit/integration** (Rust):
   - `setup_creates_tenant_when_empty`
   - `setup_returns_409_when_initialized`
   - `setup_rejects_password_mismatch`
   - `setup_status_reports_initialized_correctly`
   - `static_fallback_serves_index_for_spa_routes`

2. **Frontend unit** (Vitest):
   - Form validation in `Setup.tsx` (password length, confirm mismatch).
   - Axios interceptor injects token, handles 401.

3. **E2E** (Playwright, against ephemeral dashboard binary + ephemeral SQLite):
   - `setup_flow_initializes_db_and_redirects_to_login`
   - `login_flow_after_setup_lands_on_dashboard`
   - `provider_toggle_persists_after_reload`
   - `setup_route_unreachable_after_initialization`

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| `include_dir!` requires `web/dist` to exist at compile time | `build.rs` creates a placeholder when absent |
| Race between concurrent setup requests | Single SQL transaction with `INSERT ... WHERE NOT EXISTS` semantics on tenants count read inside a `BEGIN IMMEDIATE` |
| Embedding inflates binary size | Acceptable (<2MB typical); if it grows large, switch to `axum::Router::nest_service` over `tower-http::services::ServeDir` in dev |
| Setup endpoint abused before deploy is "ready" | Idempotent-once: 409 after first call; deploy docs note to perform setup immediately after first start |

## Migration Plan

1. Land backend `/api/setup` + `/api/setup/status` + static fallback (no UI).
2. Scaffold `web/` and ship login + setup pages; verify e2e on `192.168.50.64`.
3. Port the four CRUD pages one-by-one, each behind its own task.
4. Update `RUNBOOK.md`/`README.md` with the new first-boot flow.
5. Mark `webui-admin-bootstrap` ready for archive.
