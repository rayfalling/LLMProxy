## Context

The dashboard WebUI was bootstrapped with read-only management pages (S6.5–S6.6 of `webui-admin-bootstrap`). Backend endpoints exposed only `GET` for collections and `PUT` for a narrow set of mutations (`/providers/{id}/enabled`, `/aliases/{name}/strategy`, `/aliases/{name}/targets`, `/key-pools/{api_key_id}`, `/vision-mappings/{model_name}`). After running `/setup` the operator lands on empty pages with no way to populate them — a dead-end UX.

Login presently requires `{tenant_name, username, password}`. With single-tenant deployments being the realistic norm and `tenant_admins.username` being effectively unique anyway, the `tenant_name` field is dead weight. Removing it simplifies the login form, the API contract, and the JWT issuance code path.

The existing schema already supports the resources we need to manage (`providers`, `provider_keys`, `provider_models`, `model_aliases`, `model_alias_targets`, `api_keys`, `api_key_provider_key_pools`, `model_vision_mappings`). All FK relationships and `ON DELETE` semantics are in place. Tenant isolation is enforced at the row level via `tenant_id` columns.

Stakeholders: the only consumer is the embedded SPA. There are no external API clients to coordinate with.

## Goals / Non-Goals

**Goals:**
- Operator can complete the **entire onboarding loop in the browser**: setup → login → create provider → register provider key → register provider model → create alias → bind targets → issue tenant API key → optionally create key-pool / vision mapping → use the resulting key against the proxy.
- Login UI and API drop `tenant_name`.
- Backend create/delete handlers enforce tenant isolation (admin's `tenant_id` from JWT is the only tenant_id ever written).
- All created/deleted state survives a service restart and is reflected immediately on subsequent list calls.
- Keep existing `PUT` endpoints as-is — no breaking change to mutation paths already wired into the UI.

**Non-Goals:**
- No multi-tenant management UI (creating additional tenants from the WebUI). The setup wizard remains the only path to create a tenant.
- No bulk import UI. `scripts/seed.sql` stays as the operator escape hatch.
- No password change / admin user management UI in this change (deferred).
- No reordering or per-row enable for `provider_models` beyond what already exists.
- No proxy crate changes.

## Decisions

### D1 — Username becomes globally unique
**Decision**: replace `UNIQUE(tenant_id, username)` on `tenant_admins` with `UNIQUE(username)`. Add migration `0006_global_username_unique.sql` that drops the composite constraint and creates the global one.

**Why**: required to drop `tenant_name` from login. SQLite cannot drop a UNIQUE constraint in place — we will rebuild the table via the standard `CREATE TABLE … _new`, `INSERT … SELECT`, `DROP`, `ALTER … RENAME` recipe inside a transaction. The existing index `idx_tenant_admins_username` on `(username)` becomes redundant once the UNIQUE constraint covers it; we keep the index name only if rewriting the table preserves it (drop and let UNIQUE provide the implicit index).

**Alternatives considered**:
- Resolve duplicate usernames by prompting the user to disambiguate — rejected as terrible UX.
- Add an optional `tenant_hint` query param — rejected as it puts the burden on the user without solving anything.

### D2 — Tenant scope is always derived from JWT, never from request body
**Decision**: every new `POST` / `DELETE` handler takes `TenantAdmin` from the JWT extractor and uses `admin.tenant_id` for INSERT/DELETE WHERE clauses. Request bodies never carry `tenant_id`.

**Why**: prevents a malicious admin of tenant A from creating or deleting resources in tenant B. Mirrors how existing `PUT` handlers already operate (they all derive from the JWT).

### D3 — POST `/api/api-keys` returns the plaintext key once, then never again
**Decision**: backend stores Argon2id hash of the key; response payload includes a one-shot `plaintext_key` field. The list endpoint never returns plaintext.

**Why**: matches industry norm (AWS, OpenAI, Stripe). Frontend shows a modal with the key + a "copy" button + a warning.

**Alternative**: derive deterministic key from username/timestamp — rejected (insecure, untraceable rotation).

### D4 — Frontend forms use a shared `<ResourceCreateModal>` component
**Decision**: build one `ResourceCreateModal` that takes a JSON-Schema-like field descriptor list, renders inputs/selects/textareas, validates on submit, and calls the supplied `onSubmit(values) => Promise`. Each management page declares its field shape and reuses the same modal.

**Why**: 5 create forms with substantially identical layout. Centralised validation + error display. Easier to test (one Vitest spec covers the modal, page-specific tests just supply field configs).

**Alternative**: hand-rolled form per page — rejected (duplication, drift).

### D5 — Delete is hard for everything except `api_keys`
**Decision**: `DELETE /api/providers/{id}` etc. perform `DELETE FROM …`. Cascading FKs handle children. `DELETE /api/api-keys/{id}` performs a soft delete by `UPDATE api_keys SET status='revoked'` so historical `request_logs` keep their FK valid.

**Why**: matches what `api_keys.status` is already designed for. Hard-delete on a key whose log rows reference it would either need a CASCADE drop of audit history (no) or break referential integrity.

### D6 — Add a new top-level `ApiKeyMgmt` page rather than extending `KeyPoolMgmt`
**Decision**: create `/keys` route + `ApiKeyMgmt.tsx`. `KeyPoolMgmt` stays focused on the provider-binding side of `api_key_provider_key_pools`.

**Why**: lifecycle of an `api_keys` row (create/list/revoke) is conceptually separate from the per-provider pool selection it is later bound to. Conflating them on one page made the UI confusing during S6 review.

### D7 — Validation lives in the backend; frontend does cosmetic checks only
**Decision**: required fields, length bounds, regex (e.g. provider `name` lower-snake-case), uniqueness (e.g. duplicate alias_name in tenant) → validated server-side, return `400 Bad Request` with `{error, message, field?}`. Frontend reads `field` to highlight the offending input.

**Why**: keeps the schema authoritative server-side; frontend remains a thin client.

## Risks / Trade-offs

- **[Risk]** Global username uniqueness will block a future multi-tenant world where two tenants both want admin "alice".
  **Mitigation**: deferred concern. When/if multi-tenant admin UX is added, reintroduce `tenant_name` on login as an optional disambiguator and revert the constraint. Document this in the migration's header comment.
- **[Risk]** Deleting a provider deletes its `provider_keys` and `provider_models` via existing FK cascade, which silently invalidates `model_alias_targets` rows pointing at that provider.
  **Mitigation**: `DELETE /api/providers/{id}` first checks for referencing `model_alias_targets` and returns `409 Conflict` listing affected aliases; UI shows the conflict and prompts the user to update the alias first.
- **[Risk]** Plaintext API key transiently displayed in the browser could be captured by a malicious extension.
  **Mitigation**: warn explicitly in the reveal modal; clear from React state on close; document in RUNBOOK.
- **[Risk]** Migration `0006` rewrites `tenant_admins` — if it fails mid-way the DB is broken.
  **Mitigation**: wrap in a single transaction; ship a forward-only migration; keep the existing operator backup recipe in RUNBOOK.

## Migration Plan

1. Land the new migration; existing operator DBs auto-apply it on next dashboard startup.
2. Atomic rollout: dashboard binary + bundled SPA always ship together (already true), so the API change and UI change land at the same instant — no version skew.
3. Rollback: revert the merge commit; run a hand-written reverse SQL (provided in design.md follow-up if needed). Pre-rollback DB snapshot recommended in RUNBOOK.

## Open Questions

- **Q**: Should `POST /api/aliases` accept the initial `targets` list inline or require a separate `PUT /api/aliases/{name}/targets` after creation?
  **Working answer**: accept inline (better UX), but allow empty `targets: []` and let the existing `PUT` add them later. Backend validates that any provided `targets[].provider_id` belongs to the same tenant.
- **Q**: Confirmation dialog vs. type-the-name-to-confirm for destructive deletes?
  **Working answer**: simple confirm() for non-destructive resources; type-to-confirm only for `DELETE /api/providers/{id}` which has the largest blast radius.
