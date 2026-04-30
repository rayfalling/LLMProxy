## ADDED Requirements

### Requirement: First-Boot Setup Wizard
The dashboard SHALL provide a browser-based setup flow that initializes the
database with the first tenant and tenant-admin when no tenants exist, and
SHALL prevent re-running the flow after initialization.

#### Scenario: Fresh database serves setup wizard
- **GIVEN** the dashboard binary is started against a database where
  `SELECT COUNT(*) FROM tenants` returns `0`
- **WHEN** an operator opens any non-`/api` URL in the browser
- **THEN** the embedded SPA loads, calls `GET /api/setup/status`, receives
  `{"initialized": false}`, and routes the operator to `/setup`

#### Scenario: Setup creates tenant and admin
- **GIVEN** the database is uninitialized
- **WHEN** the operator submits `POST /api/setup` with valid `tenant_name`,
  `username`, `password` (≥ 8 chars), and matching `password_confirm`
- **THEN** the dashboard inserts one row in `tenants` and one row in
  `tenant_admins` with an Argon2id hash, in a single transaction, and responds
  `200 {"success": true}`

#### Scenario: Setup is locked after initialization
- **GIVEN** the database already contains at least one tenant
- **WHEN** any client calls `POST /api/setup`
- **THEN** the dashboard responds `409 Conflict` with body
  `{"error": "already_initialized", "message": "..."}` and does not modify any
  rows

#### Scenario: Setup rejects invalid input
- **WHEN** `POST /api/setup` is called with empty `tenant_name`, empty
  `username`, password shorter than 8 characters, OR mismatched
  `password_confirm`
- **THEN** the dashboard responds `400 Bad Request` with a structured error
  describing the failed field and does not create any rows

#### Scenario: Setup status endpoint reflects state
- **WHEN** `GET /api/setup/status` is called at any time
- **THEN** the dashboard responds `200 {"initialized": <bool>}` reflecting
  whether at least one tenant exists, and does NOT require a JWT

### Requirement: WebUI Dashboard
The dashboard binary SHALL embed and serve a single-page application that
exposes all existing dashboard control-plane APIs through a browser UI.

#### Scenario: Static SPA assets are embedded
- **GIVEN** `cargo build -p dashboard --release` succeeds with `web/dist/`
  populated by `npm run build`
- **WHEN** the resulting binary is run with no other files on disk except the
  SQLite database
- **THEN** `GET /` returns the SPA `index.html` with `Content-Type: text/html`,
  and `GET /assets/<hashed-name>.js` returns the bundled JavaScript with
  `Content-Type: application/javascript`

#### Scenario: SPA client-side routing is preserved
- **WHEN** the operator navigates directly to `/dashboard`, `/providers`,
  `/aliases`, `/keys`, `/vision`, or `/setup` (any unknown non-`/api` path)
- **THEN** the dashboard responds with the SPA `index.html` so React Router
  can resolve the route

#### Scenario: API routes are not shadowed by SPA fallback
- **WHEN** any request path begins with `/api/`
- **THEN** the dashboard routes the request through the API handlers and
  never returns the SPA `index.html`

#### Scenario: Authenticated UI consumes existing APIs
- **GIVEN** the operator has signed in via the WebUI and a JWT is stored in
  `localStorage`
- **WHEN** the WebUI calls `GET /api/providers`, `GET /api/aliases`,
  `GET /api/api-keys`, `GET /api/vision-mappings`, or
  `GET /api/stats/tenant`
- **THEN** Axios attaches `Authorization: Bearer <token>` automatically, and
  the dashboard returns tenant-scoped data

#### Scenario: Token expiry forces re-login
- **WHEN** any WebUI API call returns `401 Unauthorized`
- **THEN** the Axios response interceptor clears `jwt_token` from
  `localStorage` and the SPA navigates to `/`

### Requirement: Build & Deployment Integration
The build pipeline SHALL produce a single dashboard binary that contains the
WebUI without requiring a separate Node.js runtime at deploy time.

#### Scenario: Build script tolerates missing web/dist
- **GIVEN** `web/dist` does not exist on a clean checkout
- **WHEN** `cargo build -p dashboard` runs
- **THEN** `build.rs` creates a placeholder `web/dist/index.html` so the
  `include_dir!` macro succeeds, and the resulting binary still starts (the
  placeholder explains how to run `npm run build`)

#### Scenario: Vite dev mode proxies to dashboard
- **GIVEN** the dashboard binary is running on port 8081 and `npm run dev`
  is running on port 5173
- **WHEN** the developer opens `http://localhost:5173`
- **THEN** the Vite dev server serves the SPA and forwards every request
  whose path begins with `/api/` to `http://localhost:8081`

#### Scenario: Bare-metal deployment workflow
- **WHEN** an operator follows the documented deployment steps
  (`npm install && npm run build` in `web/`, then
  `cargo build --release -p dashboard`, then `systemctl start
  llmproxy-dashboard`)
- **THEN** opening `http://<host>:8081` on a fresh database lands on the
  setup wizard, completing the wizard enables login, and signing in unlocks
  the full WebUI without any further manual database operations
