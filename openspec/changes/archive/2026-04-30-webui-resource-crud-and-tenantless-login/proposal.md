## Why

The current WebUI ships only **read + edit-existing** flows — operators can list providers, toggle their `enabled` flag, and tweak alias strategies, but cannot **create** providers, model aliases, API keys, key-pool mappings, or vision mappings from the browser. After running setup, operators land on empty management pages with nowhere to go. Login also redundantly requires `tenant_name` even though `username` already uniquely identifies an admin in our single-setup deployment model, forcing operators to remember and re-enter the tenant name on every sign-in.

## What Changes

- **BREAKING (login API)**: `POST /api/auth/login` removes the required `tenant_name` field. Login now identifies the admin solely by `username` + `password`. The DB constraint `UNIQUE(tenant_id, username)` on `tenant_admins` is replaced with `UNIQUE(username)` (global). Existing data does not need migration on green-field installs (today only one tenant exists in deployments).
- **Login UI**: removes the tenant input, leaving only username + password.
- **Resource create endpoints (NEW)** scoped to the current admin's tenant:
  - `POST /api/providers` — create a provider (name, display_name, base_url, optional base auth header).
  - `POST /api/providers/{provider_id}/keys` — register an upstream API key for a provider, hashed via existing helper.
  - `POST /api/providers/{provider_id}/models` — register a model on a provider (model_name, optional display, capability flags).
  - `POST /api/aliases` — create a model alias (alias_name, route_strategy, initial targets list).
  - `POST /api/api-keys` — issue a new tenant API key, returns the plaintext value **once**.
  - `POST /api/key-pools` — bind a tenant api_key + provider into a key-pool mapping (selection of allowed `provider_keys`).
  - `POST /api/vision-mappings` — create a vision-routing entry (model_name → vision_parser_alias, generation_alias).
- **Resource delete endpoints (NEW)**: `DELETE` for each of the 7 collections above with proper FK cleanup (`ON DELETE` already covers most via existing schema; soft-delete only for `api_keys` via `status='revoked'`).
- **WebUI create/delete affordances**: each management page (`ProviderMgmt`, `AliasMgmt`, `KeyPoolMgmt`, `VisionMgmt`, plus a new `ApiKeyMgmt` page) gets:
  - "New …" button opening an inline form / modal.
  - Per-row delete button with confirmation dialog.
  - For `POST /api/api-keys`: a one-time reveal toast displaying the plaintext key with a copy-to-clipboard control.
- **Backend tests**: new `crates/dashboard/tests/integration.rs` cases covering create + delete happy paths and tenant isolation (admin of tenant A cannot create resources visible to tenant B).
- **Frontend tests**: Vitest specs for each new form's validation + happy path; Playwright e2e covering "setup → login → create provider → create alias targeting that provider → issue api-key → use it through proxy" full loop.
- **Docs**: README + RUNBOOK updated to describe the create-from-UI flow and remove any reference to seeding via `scripts/seed.sql` as mandatory (it remains optional bulk-import only).

## Capabilities

### New Capabilities
*(none — extending existing capability)*

### Modified Capabilities
- `dashboard-webui`: Login no longer takes `tenant_name`; new resource-create / resource-delete requirements added; per-resource UI form requirements added.

## Impact

- **API surface**: 7 new `POST` routes + 7 new `DELETE` routes under `/api/`; `LoginRequest` schema breaking-change (drops `tenant_name`).
- **DB schema**: new migration `0006_global_username_unique.sql` rewrites the `tenant_admins` UNIQUE constraint.
- **Backend code**: `crates/dashboard/src/auth.rs` (login query), `crates/dashboard/src/handlers.rs` (create/delete handlers), `crates/dashboard/src/main.rs` (router), `crates/dashboard/tests/integration.rs` (regression).
- **Frontend code**: `web/src/pages/Login.tsx` (drop tenant field), `web/src/pages/{ProviderMgmt,AliasMgmt,KeyPoolMgmt,VisionMgmt}.tsx`, new `web/src/pages/ApiKeyMgmt.tsx`, new shared modal/form components, `web/src/api/*.ts` clients, `web/tests/`, `web/e2e/`.
- **Docs**: `README.md`, `RUNBOOK.md`.
- **No impact** on the proxy crate (`crates/proxy`) or any non-dashboard control-plane behavior.
- **Compatibility**: the `tenant_name` removal is a wire-format breaking change; since this product has no published clients besides the embedded SPA, mitigation is trivial (rebuild SPA atomically with backend).
