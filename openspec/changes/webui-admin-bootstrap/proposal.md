## Why

Phase-1 (`rust-openrouter-proxy-dashboard`) shipped REST control-plane APIs only.
Operators currently have no way to:

1. Sign in to a freshly deployed instance — the database starts empty, with zero
   tenants/admins, so login is impossible without manual SQL inserts.
2. Manage providers, aliases, key-pools, vision mappings without crafting cURL
   calls and JWT tokens by hand.

This blocks the project from being installable on bare metal in any practical
sense. The deployment validation on `192.168.50.64` exposed both gaps directly.

## Goals

1. Provide a first-boot **setup wizard** so a fresh deployment can be initialized
   through the browser without touching SQLite or running scripts.
2. Provide a **WebUI dashboard** that exposes every existing dashboard REST API
   as an interactive screen, scoped to the authenticated tenant admin.
3. Ship the WebUI as part of the dashboard binary (single binary deploy still
   works) and keep dev ergonomics (Vite hot reload against the API).

## Non-Goals

- Multi-language i18n (English only in this change).
- Role-based access control inside the UI (single tenant-admin role for now).
- Server-side rendering — SPA only.
- Full design-system theming — Tailwind defaults only.

## Scope

In scope:
- Frontend project at repo root `web/` (React 18 + TypeScript + Vite +
  Tailwind).
- Setup flow:
  - Dashboard exposes `POST /api/setup` that creates the first tenant + admin
    when the database is empty; rejected with 409 once seeded.
  - Empty-DB detection serves the SPA `/setup` route on first visit.
- Auth flow: existing `POST /api/auth/login` consumed by the WebUI; JWT stored
  in `localStorage`; expired-token handling redirects to login.
- Dashboard pages: home (stats + recent failover events), providers, aliases,
  key-pool mappings, vision mappings.
- Build integration: `cargo build -p dashboard` triggers `web/dist` to be
  embedded via `include_dir!`; `web/dist` is `.gitignore`d.
- Dev mode: Vite dev server proxies `/api` to the running dashboard binary on
  `:8081`.
- Playwright E2E: setup, login, provider toggle, alias edit.
- Optional `scripts/seed.sql` for demo data after setup.

Out of scope:
- New dashboard REST endpoints (only `/api/setup` is added).
- Modifying the proxy crate.

## Deployment Story

1. `systemctl start llmproxy-dashboard` on a clean host.
2. Operator opens `http://host:8081/`.
3. Empty-DB middleware redirects to `/setup`; operator submits tenant name,
   admin username, password.
4. Backend creates tenant + admin (Argon2 hash), responds success.
5. UI redirects to `/` (login). Operator signs in.
6. Subsequent visits skip the wizard.

## Decisions (Confirmed)

1. Frontend lives in repo root `web/` (NOT under `crates/`), so it is not
   compiled by `cargo build` directly.
2. Static assets are embedded into the dashboard binary at build time via
   `include_dir!("../../web/dist")`. A build script ensures the directory exists
   even if `npm run build` was skipped (placeholder index.html).
3. Setup endpoint is idempotent-once: succeeds while `tenants` is empty, returns
   409 afterwards. No "reset" path in this change.
4. JWT lifetime, password hashing parameters, and tenant scoping reuse the
   existing dashboard auth code without modification.
5. Frontend stack: React 18 + TypeScript + Vite + Tailwind + Axios + React
   Router v6. Playwright for E2E.

## Open Questions

- None for this change. Future work (RBAC, SSO, audit log UI) tracked separately.
