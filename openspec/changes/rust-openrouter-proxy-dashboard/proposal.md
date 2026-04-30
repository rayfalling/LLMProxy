## Why

We need a Rust-based LLM proxy similar to OpenRouter that can:
- Accept both OpenAI-compatible and Claude-compatible client protocols.
- Provide a dashboard for operations and policy control.
- Support image input parsing where vision parsing model and text LLM can be configured separately.
- Default to one multimodal model for both vision and text when vision is supported; otherwise fall back to split pipeline.
- Allow provider-level and model-level enable/disable from dashboard.
- Aggregate equivalent models across providers and automatically fail over when one provider is unavailable (for example billing failure on one DeepSeek provider).
- Support per-model outbound network proxy settings to bypass regional connectivity restrictions.

## Goals

1. Build a production-ready Rust proxy service with clear provider abstraction.
2. Prioritize complete Claude interface compatibility first, then complete OpenAI interface compatibility.
3. Implement multimodal image parsing pipeline with configurable model split:
   - image parser model
   - generation LLM model
4. Build dashboard for:
   - provider force-enable/disable
   - provider model allow-list/deny-list
   - route policy visualization and editing
   - health and failover status
5. Implement model aggregation with automatic migration:
   - logical model aliases map to multiple provider backends
   - fixed-priority fallback sequence
6. Support multi-tenant isolation:
   - per-API-key tenant scope for config and metrics visibility
   - per-key quota, rate-limit, and access control
7. Persist config and statistics in database storage.
8. Allow switchable routing objective strategy (reliability/latency/cost).

## Non-Goals (Phase 1)

- Full billing settlement and invoice export.
- Full tenant self-service portal.
- Fine-grained prompt governance DSL.
- Multi-region active-active deployment.

## Scope (Phase 1)

In scope:
- Claude full interface compatibility first (all stable + beta), OpenAI full interface compatibility second (chat, responses, images, audio, realtime).
- Unified internal canonical request/response schema.
- Provider adapters for 7 official providers: OpenAI, Anthropic, Copilot, Xiaomi, Google, OpenRouter, DeepSeek.
- Dashboard with multi-tenant controls and API-key-scoped data view (role differentiation deferred).
- Config and statistics persistence in SQLite.
- Observability baseline: request logs, latency, provider health, and tenant-scoped statistics.
- Per-model outbound proxy configuration.
- Key pool mapping and per-key quota/rate-limit/ACL enforcement.
- Tenant admin JWT authentication; SSO deferred.

Out of scope:
- Complex billing center.
- Deep analytics warehouse.

## Milestones

M1. Foundation and Claude protocol bridge
- Rust workspace scaffold
- SQLite schema and migration baseline
- Claude full interface adapters (stable + beta) and contract tests
- Canonical request/response model
- Basic upstream adapter trait

M2. OpenAI compatibility and multimodal routing
- OpenAI full interface adapters (chat/responses/images/audio/realtime) and contract tests
- Image parsing chain: default same-model multimodal, fallback split pipeline
- Tenant JWT auth and API-key-scoped isolation

M3. Provider build-out and failover
- 7 provider adapters: OpenAI, Anthropic, Copilot, Xiaomi, Google, OpenRouter, DeepSeek
- Logical model registry
- Fixed-priority provider failover
- Trigger classes: insufficient balance, 429, 5xx, timeout, model offline, manual disable
- Per-model outbound proxy support

M4. Hardening
- End-to-end tests (OpenAI/Claude/streaming/image/failover)
- Rate-limit and key policy
- Multi-tenant dashboard isolation and audit logs
- Database persistence migration and backup basics

## Success Criteria

- OpenAI-compatible and Claude-compatible clients can call the proxy without client code change.
- Image+text requests use default same-model multimodal path, and auto-fallback to split pipeline for non-vision models.
- Operator can disable a provider in dashboard and traffic migrates according to policy.
- Equivalent model fallback works for key failure classes: balance insufficiency, 429, 5xx, timeout, model offline, manual disable.
- Tenant API keys only access their own config scope and metrics scope.
- Routing objective can be switched by policy configuration.
- Model-specific outbound proxy settings are effective at runtime.

## Decisions (Confirmed)

1. Claude full interfaces: include all stable and beta headers/features in Phase 1.
2. OpenAI full interfaces: include responses, images, audio, and realtime in Phase 1.
3. Database: SQLite for Phase 1 (can migrate to PostgreSQL later).
4. Dashboard auth: tenant admin JWT in Phase 1; SSO deferred to later phase.
5. Multi-tenant isolation: API key level only in Phase 1; sub-roles deferred.
6. Provider set Phase 1: 7 official providers —
   - OpenAI
   - Anthropic
   - Microsoft Copilot (Azure OpenAI compatible endpoint)
   - Xiaomi (Mi AI)
   - Google (Gemini API)
   - OpenRouter
   - DeepSeek official
