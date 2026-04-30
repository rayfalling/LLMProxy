## 1. Schema migration & login

- [ ] 1.1 Add migration `migrations/0006_global_username_unique.sql` that rebuilds `tenant_admins` with `UNIQUE(username)` (transactional `CREATE TABLE _new` → `INSERT … SELECT` → `DROP` → `ALTER … RENAME`); preserve all existing rows; add a header comment explaining the multi-tenant rollback path.
- [ ] 1.2 Run `cargo build -p dashboard` against a fresh DB and against a DB with seeded `tenant_admins` rows; assert migration is idempotent + non-destructive.
- [ ] 1.3 Drop `tenant_name` from `LoginRequest` in `crates/dashboard/src/auth.rs`; rewrite the SQL to `SELECT … FROM tenant_admins ta JOIN tenants t ON t.id = ta.tenant_id WHERE ta.username = ? AND ta.status = 'active' AND t.status = 'active'`; keep response payload unchanged.
- [ ] 1.4 Update existing integration tests in `crates/dashboard/tests/integration.rs` and `setup.rs`/`static_fallback.rs` to drop `tenant_name` from login bodies; add a new test asserting that login with a unique username succeeds and that creating a second admin with the same username returns `409 username_taken`.
- [ ] 1.5 Update `web/src/pages/Login.tsx` to remove the tenant input and the `tenant_name` field from the request body; update `web/src/api/auth.ts`.
- [ ] 1.6 Update `web/tests/Login.spec.tsx` (Vitest) to assert exactly two inputs render and submission shape matches new schema.

## 2. Backend create/delete handlers

- [ ] 2.1 Add `POST /api/providers` and `DELETE /api/providers/{provider_id}` handlers in `crates/dashboard/src/handlers.rs`. Validate `name` regex `^[a-z][a-z0-9_-]{1,31}$` and non-empty `base_url`; return `400 {error: "invalid_field", field}` on failure; on delete, pre-check for `model_alias_targets` references and return `409 in_use` listing alias names.
- [ ] 2.2 Add `POST /api/providers/{provider_id}/keys` and `DELETE /api/providers/{provider_id}/keys/{key_id}`; reuse existing key-hashing helper; never return plaintext.
- [ ] 2.3 Add `POST /api/providers/{provider_id}/models` and `DELETE /api/providers/{provider_id}/models/{model_name}`.
- [ ] 2.4 Add `POST /api/aliases` (accept inline `targets: []`, validate target provider_ids are in admin's tenant) and `DELETE /api/aliases/{alias_name}`. Wrap multi-row inserts in a transaction.
- [ ] 2.5 Add `POST /api/api-keys` and `DELETE /api/api-keys/{key_id}` (soft-delete by `status = 'revoked'`); generate plaintext via existing helper; return plaintext only in POST response; ensure list/get endpoints scrub plaintext.
- [ ] 2.6 Add `POST /api/key-pools` and `DELETE /api/key-pools/{api_key_id}/{provider_id}`; validate `api_key_id`, `provider_id`, and every `allowed_provider_key_ids` entry belongs to the admin's tenant; reject otherwise with `400`.
- [ ] 2.7 Add `POST /api/vision-mappings` and `DELETE /api/vision-mappings/{model_name}`; tenant-scoped.
- [ ] 2.8 Wire all 14 new routes in `crates/dashboard/src/main.rs` with the existing `auth_layer`.
- [ ] 2.9 Ensure every new handler enforces tenant isolation by deriving `tenant_id` from `TenantAdmin` extractor only; cross-tenant attempts return `404 Not Found` (never leak existence).

## 3. Backend tests

- [ ] 3.1 Extend `crates/dashboard/tests/integration.rs`: happy-path POST + DELETE for each of the 7 resources.
- [ ] 3.2 Add tenant-isolation tests: seed two tenants, sign in as admin A, assert POST/DELETE against tenant-B-owned ids return `404`.
- [ ] 3.3 Add validation tests: invalid provider name, empty base_url, duplicate alias_name, target provider_id from wrong tenant, all return correct `400` shapes.
- [ ] 3.4 Add API-key reveal-once test: POST returns `plaintext_key`; GET list never includes it; DELETE flips `status` to `'revoked'`; subsequent proxy auth using the revoked key fails (proxy-side check covered by existing proxy tests, here just assert the row state).
- [ ] 3.5 Add provider-deletion-blocked test: seed alias with target → DELETE provider → assert `409 in_use` and zero deletions.
- [ ] 3.6 `cargo test --workspace` green.

## 4. Frontend shared components

- [ ] 4.1 Build `web/src/components/ResourceCreateModal.tsx` accepting a `fields: FieldDescriptor[]` prop (name, label, type: text|password|select|checkbox-list|repeater, required, pattern?, options?), an `onSubmit(values)` callback, and rendering a modal with validation + submit + cancel.
- [ ] 4.2 Build `web/src/components/ConfirmDeleteDialog.tsx` supporting both simple confirm and type-the-name-to-confirm modes.
- [ ] 4.3 Build `web/src/components/RevealOnceModal.tsx` for displaying plaintext API keys with a copy-to-clipboard button + warning banner; clears state on close.
- [ ] 4.4 Add Vitest specs for each component covering validation paths, copy-to-clipboard fallback, and field-error highlighting from server `field` payloads.

## 5. Frontend management pages

- [ ] 5.1 Update `web/src/pages/ProviderMgmt.tsx` to wire a "New Provider" button (uses `ResourceCreateModal`) and a per-row "Delete" button (uses `ConfirmDeleteDialog` in type-the-name mode); refresh list on success; surface `409 in_use` with the list of aliases.
- [ ] 5.2 Inside the provider detail row, add nested "Add Key" and "Add Model" actions plus per-row delete for keys and models.
- [ ] 5.3 Update `web/src/pages/AliasMgmt.tsx` to add "New Alias" (with `targets` repeater) and per-row "Delete".
- [ ] 5.4 Create `web/src/pages/ApiKeyMgmt.tsx` for `/keys`: list (label, prefix, status, created_at), "Issue New Key" button (uses `ResourceCreateModal` then `RevealOnceModal`), "Revoke" button per row.
- [ ] 5.5 Update `web/src/pages/KeyPoolMgmt.tsx`: add "New Mapping" button (select api_key, select provider, multi-check provider_keys) and "Delete" per row.
- [ ] 5.6 Update `web/src/pages/VisionMgmt.tsx` to add "New Mapping" button and "Delete" per row.
- [ ] 5.7 Add `/keys` route to `App.tsx` and link from the side nav.
- [ ] 5.8 Update `web/src/api/*.ts` with the new POST/DELETE clients.

## 6. Frontend tests

- [ ] 6.1 Vitest: each page's create modal validates required fields and submits the right payload; delete dialog appears and calls the right endpoint.
- [ ] 6.2 Vitest: `RevealOnceModal` clears state on close and copies plaintext via the Clipboard API stub.
- [ ] 6.3 Playwright e2e in `web/e2e/onboarding.spec.ts`: setup → login (no tenant) → create provider → add provider-key → add provider-model → create alias targeting it → issue api-key → assert plaintext shown once → revoke → assert proxy `/v1/chat/completions` rejects the revoked key.
- [ ] 6.4 Playwright: cross-tenant isolation smoke (seed two tenants via SQL, login as A, assert provider list does not include B's rows; attempting to DELETE B's id surfaces a "not found" error toast).

## 7. Docs

- [ ] 7.1 Update `README.md` "Getting Started" section: replace any reference to seeding via SQL with a step-by-step UI walk-through (setup → login → New Provider → New Alias → Issue API Key); keep `scripts/seed.sql` mentioned as an optional bulk-import path.
- [ ] 7.2 Update `RUNBOOK.md`: section on revoking a leaked API key now points at the WebUI revoke button; section on rotating a provider key documents the "delete + add" flow; add a note about the one-time plaintext reveal.
- [ ] 7.3 Update OpenSpec change tasks file as work completes; mark each item `[x]` after verification.

## 8. Validation & archive

- [ ] 8.1 `cargo test --workspace` — all green.
- [ ] 8.2 `cd web && npm run test` (Vitest) — all green.
- [ ] 8.3 `cd web && npm run test:e2e` (Playwright) — all green against a freshly built local dashboard.
- [ ] 8.4 Bare-metal validation on `192.168.50.64`: deploy fresh build, complete setup wizard manually in browser, walk through full create flow for one provider + one alias + one api-key, validate proxy chat completion through the new key (real upstream optional — at minimum `/v1/models` listing), confirm revoke takes effect.
- [ ] 8.5 `openspec validate webui-resource-crud-and-tenantless-login --strict` clean.
- [ ] 8.6 Archive: `openspec archive webui-resource-crud-and-tenantless-login`.
