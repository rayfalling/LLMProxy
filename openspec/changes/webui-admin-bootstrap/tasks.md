## 1. Backend — Setup Endpoint

- [x] 1.1 Add `crates/dashboard/src/bootstrap.rs` with `is_initialized(pool)` helper.
- [x] 1.2 Implement `POST /api/setup` handler (validation + Argon2 + transactional insert).
- [x] 1.3 Implement `GET /api/setup/status` returning `{"initialized": bool}`.
- [x] 1.4 Wire both routes into `dashboard::router()` (no JWT required).
- [x] 1.5 Unit tests: empty DB success, 409 when already initialized, password mismatch, password too short.
- [x] 1.6 Update `dashboard::tests::integration` to cover setup → login round trip.

## 2. Backend — Static Asset Serving

- [x] 2.1 Add `include_dir`, `mime_guess` to `crates/dashboard/Cargo.toml`.
- [x] 2.2 Create `crates/dashboard/build.rs` that ensures `web/dist/index.html` placeholder exists.
- [x] 2.3 Add `crates/dashboard/src/static_assets.rs` with `WEB_DIST` `Dir`.
- [x] 2.4 Add SPA fallback service: `/api/*` → API router, everything else → embedded asset or `index.html`.
- [x] 2.5 Integration test: GET `/` returns HTML; GET `/dashboard` returns same `index.html`; GET `/assets/foo.js` (when present) returns JS with correct mime.

## 3. Frontend — Project Scaffold (`web/`)

- [x] 3.1 Verify `web/package.json`, `vite.config.ts`, `tsconfig.json`, Tailwind/PostCSS configs (already created in prior step; reconcile after move).
- [x] 3.2 Add `web/playwright.config.ts` and `web/tests/` directory.
- [x] 3.3 Add npm scripts: `dev`, `build`, `preview`, `test:unit`, `test:e2e`.
- [x] 3.4 Add `web/.gitignore` for `node_modules`, `dist`, `playwright-report`, `test-results`.
- [x] 3.5 Verify `npm install && npm run build` produces `web/dist/index.html` + assets locally.

## 4. Frontend — Setup & Auth Flow

- [x] 4.1 `src/api/client.ts` — Axios instance with JWT interceptor and 401 handling.
- [x] 4.2 `src/api/types.ts` — TS interfaces for setup, auth, providers, aliases, stats.
- [x] 4.3 `src/pages/Setup.tsx` — form, validation, success redirect.
- [x] 4.4 `src/pages/Login.tsx` — form, error display, JWT persistence.
- [x] 4.5 `src/components/ProtectedRoute.tsx` + `App.tsx` route table with setup-status guard.
- [x] 4.6 Vitest cases for Setup validation + Axios interceptors.

## 5. Frontend — Dashboard CRUD Pages

- [x] 5.1 `Dashboard.tsx` — tenant stats cards + last 10 failover events.
- [x] 5.2 `ProviderMgmt.tsx` — list providers, toggle enabled, expand to model rows with per-model toggle.
- [x] 5.3 `AliasMgmt.tsx` — list aliases, edit `route_strategy` (target reorder UI deferred; backend `PUT /api/aliases/<a>/targets` ready).
- [x] 5.4 `KeyPoolMgmt.tsx` — list downstream API keys + their provider key associations (read-only view; edit via API in this iteration).
- [x] 5.5 `VisionMgmt.tsx` — list model→vision_parser/generation_alias mappings, inline edit + save.
- [x] 5.6 `AppLayout.tsx` — top nav linking the five pages, logout button.

## 6. End-to-End Tests (Playwright)

- [x] 6.1 `tests/e2e.spec.ts` — fresh DB → setup wizard → success → land on `/`.
- [x] 6.2 `tests/e2e.spec.ts` — login with seeded admin → land on `/dashboard`.
- [x] 6.3 `tests/e2e.spec.ts` — `/providers` page loads with auth (CRUD-toggle covered by Vitest + tasks.7 seed).
- [x] 6.4 `tests/e2e.spec.ts` — `/aliases` page loads with auth (full target-reorder UI deferred per Section 5 note).
- [x] 6.5 `tests/e2e.spec.ts` — `/setup` redirects to `/` once initialized.
- [x] 6.6 CI script `scripts/e2e.ps1` (Windows) and `scripts/e2e.sh` (Linux) that builds, launches dashboard with temp SQLite, runs Playwright, tears down.

## 7. Documentation & Deployment

- [x] 7.1 Update root `README.md`: dev workflow (Rust + npm), production build (`npm run build` then `cargo build`), first-boot setup screenshots/steps.
- [x] 7.2 Update `RUNBOOK.md`: incident steps for "DB wiped" and "setup endpoint inaccessible".
- [x] 7.3 Add `scripts/seed.sql` for optional demo data (provider, models, alias, demo api_key + key-pool binding) plus `scripts/hash-key.{ps1,sh}` helpers. Tenant/admin creation deliberately delegated to the WebUI `/setup` wizard.
- [x] 7.4 No `deploy/` directory in this repo; systemd guidance lives inline in `RUNBOOK.md` §0 ("Database Wiped / Fresh Host") and `README.md` §First-Boot Setup, both calling out the `/setup` first-visit step.
- [ ] 7.5 Bare-metal validation on `192.168.50.64` — DEFERRED to operator. Steps: deploy fresh build, browse to `:8081/setup`, complete wizard, optionally apply `scripts/seed.sql`, run a chat completion through the proxy with the demo API key.

## 8. Release

- [x] 8.1 All Rust unit/integration tests green (`cargo test --workspace`).
- [x] 8.2 All Vitest + Playwright tests green.
- [x] 8.3 Single `cargo build --release -p dashboard` produces a binary that serves the SPA correctly when run alone. Verified: fresh DB, `/` + `/setup` deep-link both return embedded `index.html` (484 B), `/assets/index-*.js` returns 228 KB with `text/javascript`, `GET /api/setup/status` returns `{initialized:false}`.
- [ ] 8.4 Update OpenSpec change status; archive after operator validation on bare metal.
