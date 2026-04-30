## MODIFIED Requirements

### Requirement: WebUI Dashboard
The dashboard binary SHALL embed and serve a single-page application that
exposes all existing dashboard control-plane APIs through a browser UI,
SHALL allow operators to **create and delete** providers, provider keys,
provider models, model aliases, tenant API keys, key-pool mappings, and
vision mappings entirely from the browser, and SHALL authenticate operators
using only `username` + `password` (no `tenant_name`).

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
  `/aliases`, `/keys`, `/key-pools`, `/vision`, or `/setup` (any unknown
  non-`/api` path)
- **THEN** the dashboard responds with the SPA `index.html` so React Router
  can resolve the route

#### Scenario: API routes are not shadowed by SPA fallback
- **WHEN** any request path begins with `/api/`
- **THEN** the dashboard routes the request through the API handlers and
  never returns the SPA `index.html`

#### Scenario: Tenantless login succeeds with username + password
- **GIVEN** exactly one `tenant_admins` row exists with the supplied
  `username` and a matching Argon2id password hash
- **WHEN** the operator submits `POST /api/auth/login` with body
  `{"username": "...", "password": "..."}` (no `tenant_name`)
- **THEN** the dashboard responds `200` with a JWT whose `tenant_id` claim
  equals the admin's tenant_id, and the response body contains
  `{token, token_type:"Bearer", expires_in, tenant_id, username}`

#### Scenario: Login UI does not show a tenant input
- **WHEN** the operator visits `/login` (or is redirected there from any
  protected route)
- **THEN** the rendered form contains exactly two inputs (`username`,
  `password`) plus a submit button, and zero tenant-related fields

#### Scenario: Token expiry forces re-login
- **WHEN** any WebUI API call returns `401 Unauthorized`
- **THEN** the Axios response interceptor clears `jwt_token` from
  `localStorage` and the SPA navigates to `/login`

## ADDED Requirements

### Requirement: Global Username Uniqueness
The dashboard schema SHALL enforce that `tenant_admins.username` is globally
unique so that login can identify an admin without a tenant qualifier.

#### Scenario: Migration enforces global UNIQUE(username)
- **GIVEN** an existing dashboard database with the prior
  `UNIQUE(tenant_id, username)` constraint
- **WHEN** the dashboard binary starts and runs migrations
- **THEN** migration `0006_global_username_unique.sql` rebuilds
  `tenant_admins` with `UNIQUE(username)` inside a single transaction,
  preserving every existing row

#### Scenario: Setup rejects a username already taken globally
- **GIVEN** a `tenant_admins` row already exists with `username = 'alice'`
- **WHEN** `POST /api/setup` is somehow re-issued or a new admin creation
  endpoint receives `username = 'alice'`
- **THEN** the operation returns `409 Conflict` with
  `{"error": "username_taken", ...}` and inserts no rows

### Requirement: Provider Lifecycle From WebUI
The dashboard SHALL expose `POST /api/providers` and
`DELETE /api/providers/{provider_id}` so operators can create and remove
providers entirely from the browser.

#### Scenario: Authenticated admin creates a provider
- **GIVEN** a valid admin JWT
- **WHEN** the SPA submits
  `POST /api/providers` with
  `{name, display_name, base_url, auth_header_name?, auth_header_template?}`
- **THEN** the dashboard inserts one row in `providers` with
  `tenant_id = <jwt.tenant_id>` and `enabled = 1`, returns `201` with the
  inserted row, and a subsequent `GET /api/providers` includes the new row

#### Scenario: Provider creation rejects invalid name
- **WHEN** `POST /api/providers` is called with `name` not matching
  `^[a-z][a-z0-9_-]{1,31}$` or empty `base_url`
- **THEN** the dashboard responds `400 Bad Request` with
  `{error: "invalid_field", message: ..., field: "name"}` (or `field: "base_url"`)
  and inserts no rows

#### Scenario: Provider deletion is blocked while aliases reference it
- **GIVEN** at least one `model_alias_targets` row references
  `provider_id = <pid>`
- **WHEN** `DELETE /api/providers/<pid>` is called
- **THEN** the dashboard responds `409 Conflict` with
  `{error: "in_use", message: ..., aliases: ["<alias_name>", ...]}` and
  deletes no rows

#### Scenario: Unreferenced provider deletion succeeds and cascades children
- **GIVEN** no `model_alias_targets` rows reference `provider_id = <pid>`
- **WHEN** `DELETE /api/providers/<pid>` is called by an admin of the
  provider's tenant
- **THEN** the dashboard deletes the `providers` row and FK-cascades to
  child `provider_keys` and `provider_models` rows, returning `204`

#### Scenario: Cross-tenant deletion is rejected
- **WHEN** an admin of tenant A calls `DELETE /api/providers/<pid>` for a
  provider belonging to tenant B
- **THEN** the dashboard responds `404 Not Found` and deletes no rows

### Requirement: Provider Key And Model Lifecycle From WebUI
The dashboard SHALL allow creating and deleting upstream provider keys and
provider models from the browser.

#### Scenario: Add a provider key
- **GIVEN** an admin JWT and an existing provider in the same tenant
- **WHEN** the SPA submits
  `POST /api/providers/{provider_id}/keys` with `{label, plaintext_key}`
- **THEN** the dashboard hashes the key (existing helper), stores a row in
  `provider_keys`, and responds `201` with the row's `id` and `label`
  (never the plaintext)

#### Scenario: Delete a provider key
- **WHEN** `DELETE /api/providers/{provider_id}/keys/{key_id}` is called by
  the owning tenant's admin
- **THEN** the dashboard removes the `provider_keys` row and responds `204`

#### Scenario: Add a provider model
- **WHEN** `POST /api/providers/{provider_id}/models` is called with
  `{model_name, display_name?, capability_flags?}` by the owning tenant's
  admin
- **THEN** the dashboard inserts the `provider_models` row (`enabled = 1`)
  and responds `201`

#### Scenario: Delete a provider model
- **WHEN** `DELETE /api/providers/{provider_id}/models/{model_name}` is
  called by the owning tenant's admin
- **THEN** the dashboard removes the row and responds `204`

### Requirement: Model Alias Lifecycle From WebUI
The dashboard SHALL allow creating and deleting model aliases (and their
target lists) from the browser.

#### Scenario: Create alias with inline targets
- **WHEN** the SPA submits
  `POST /api/aliases` with
  `{alias_name, route_strategy, targets: [{provider_id, provider_model, weight?, priority?}, ...]}`
- **THEN** the dashboard inserts one row in `model_aliases` and N rows in
  `model_alias_targets`, all in one transaction, returns `201` with the
  populated alias, and rejects with `400` if any target's provider_id is
  not in the admin's tenant

#### Scenario: Create alias without targets
- **WHEN** `POST /api/aliases` is called with `targets: []`
- **THEN** the dashboard creates the alias row only, returns `201`, and
  the operator may add targets later via the existing
  `PUT /api/aliases/{name}/targets`

#### Scenario: Delete alias
- **WHEN** `DELETE /api/aliases/{alias_name}` is called by the owning
  tenant's admin
- **THEN** the dashboard removes the `model_aliases` row, FK-cascades to
  `model_alias_targets`, and responds `204`

### Requirement: Tenant API Key Lifecycle From WebUI
The dashboard SHALL allow operators to issue and revoke tenant API keys
from the browser. Plaintext keys SHALL be revealed only once.

#### Scenario: Issue API key returns plaintext exactly once
- **WHEN** the SPA submits `POST /api/api-keys` with `{label}`
- **THEN** the dashboard generates a random key, stores its Argon2id hash
  in `api_keys`, and responds `201` with `{id, label, plaintext_key, prefix, created_at}`
  where `plaintext_key` is the only place the unhashed value appears

#### Scenario: List omits plaintext
- **WHEN** `GET /api/api-keys` is called
- **THEN** the response items contain `id, label, prefix, status, created_at, last_used_at`
  and **never** `plaintext_key`

#### Scenario: Revoke API key (soft delete)
- **WHEN** `DELETE /api/api-keys/{key_id}` is called by the owning tenant's
  admin
- **THEN** the dashboard sets `api_keys.status = 'revoked'` (does not delete
  the row) and responds `204`; subsequent proxy requests using that key
  fail authentication

#### Scenario: Reveal modal warns and clears state
- **WHEN** the SPA receives a `plaintext_key` from `POST /api/api-keys`
- **THEN** the SPA opens a one-time modal showing the key, a copy-to-clipboard
  control, and a warning that the key will not be shown again, and clears
  the value from React state when the modal is closed

### Requirement: Key-Pool Mapping And Vision Mapping Lifecycle From WebUI
The dashboard SHALL allow operators to create and delete
`api_key_provider_key_pools` rows (binding a tenant API key to a provider's
selected upstream keys) and `model_vision_mappings` rows entirely from the
browser.

#### Scenario: Create key-pool mapping
- **WHEN** the SPA submits
  `POST /api/key-pools` with
  `{api_key_id, provider_id, allowed_provider_key_ids: [...]}`
- **THEN** the dashboard validates that all referenced ids belong to the
  admin's tenant, inserts the row in `api_key_provider_key_pools`, and
  responds `201`

#### Scenario: Delete key-pool mapping
- **WHEN** `DELETE /api/key-pools/{api_key_id}/{provider_id}` is called
- **THEN** the dashboard removes the row and responds `204`

#### Scenario: Create vision mapping
- **WHEN** `POST /api/vision-mappings` is called with
  `{model_name, vision_parser_alias, generation_alias}`
- **THEN** the dashboard inserts a `model_vision_mappings` row scoped to
  the admin's tenant and responds `201`

#### Scenario: Delete vision mapping
- **WHEN** `DELETE /api/vision-mappings/{model_name}` is called by the
  owning tenant's admin
- **THEN** the dashboard removes the row and responds `204`

### Requirement: Resource Create UI Affordances
The SPA SHALL surface a "New …" action and a per-row delete control on
each management page so the entire resource lifecycle is reachable through
the browser without manual SQL.

#### Scenario: Provider page exposes create + delete
- **GIVEN** the operator is signed in and on `/providers`
- **WHEN** the page renders
- **THEN** the page contains a "New Provider" button that opens a modal
  with `name`, `display_name`, `base_url`, `auth_header_name?`,
  `auth_header_template?` inputs, and each provider row shows a "Delete"
  button that opens a type-the-name-to-confirm dialog

#### Scenario: Alias page exposes create + delete
- **GIVEN** the operator is on `/aliases`
- **WHEN** the page renders
- **THEN** the page contains a "New Alias" button that opens a modal with
  `alias_name`, `route_strategy` (select), and a repeater for `targets`,
  and each alias row exposes a "Delete" button with a simple confirmation

#### Scenario: API key page exposes create with one-time reveal
- **GIVEN** the operator is on `/keys`
- **WHEN** the operator clicks "Issue New Key", supplies a `label`, and
  submits
- **THEN** the SPA shows the plaintext key in a one-time modal with a copy
  control and a warning, and the new key appears in the list with its
  `prefix` masked thereafter

#### Scenario: Key-pool page exposes create + delete
- **GIVEN** the operator is on `/key-pools`
- **WHEN** the page renders with at least one tenant API key and one
  provider in the tenant
- **THEN** the page contains a "New Mapping" button that lets the operator
  pick an API key, a provider, and check off `provider_keys`, and each
  mapping row exposes a "Delete" button

#### Scenario: Vision mapping page exposes create + delete
- **GIVEN** the operator is on `/vision`
- **WHEN** the page renders
- **THEN** the page contains a "New Mapping" button with `model_name`,
  `vision_parser_alias`, `generation_alias` inputs, and each row exposes
  a "Delete" button

#### Scenario: Form validation surfaces server-reported field
- **WHEN** any create form submission triggers a `400 Bad Request` whose
  body contains `{field: "<name>"}`
- **THEN** the SPA highlights the matching input and displays the
  `message` next to it
