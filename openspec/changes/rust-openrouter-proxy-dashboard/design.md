## Overview

Architecture uses a protocol-edge + canonical-core + provider-adapter pattern.
This keeps external protocol compatibility independent from provider-specific behavior.

## Components

1. Edge API Layer
- Claude-compatible full-interface handlers (priority)
- OpenAI-compatible full-interface handlers
- Streaming bridge (SSE/chunk translation)

2. Canonical Core
- Canonical request model
- Canonical response model
- Unified error taxonomy
- Policy evaluation context

3. Routing Engine
- Model alias resolution
- Provider selection strategy (switchable objectives)
- Retry and fallback orchestration
- Circuit breaker and health scoring

4. Provider Adapters (Phase 1 — all 7)
- OpenAI adapter (chat, responses, images, audio, realtime)
- Anthropic adapter (full Claude API including beta headers)
- Microsoft Copilot adapter (Azure OpenAI endpoint)
- Xiaomi adapter (Mi AI API)
- Google adapter (Gemini API)
- OpenRouter adapter
- DeepSeek official adapter

5. Tenant and Auth Layer
- API key to tenant mapping
- Tenant admin JWT authentication (SSO deferred)
- Tenant-scoped quota, rate-limit, ACL (sub-roles deferred)
- Tenant data scope enforcement for dashboard and metrics

6. Multimodal Pipeline
- Default same-model multimodal execution when model supports vision
- Fallback vision parse step with separate provider/model when model does not support vision
- Prompt assembly step
- Generation model execution step

7. Control Plane
- Dashboard API
- Provider and model registry config
- Policy config and feature flags
- Audit events

8. Storage and Telemetry
- Config store
- Runtime health cache
- Request logs and metrics (tenant-scoped)
- Persistent relational storage for config and statistics

9. Egress Proxy Layer
- Per-model outbound proxy configuration
- Provider request transport binding to model-level proxy settings

## Request Flow

1. Client request enters OpenAI or Claude endpoint.
2. Protocol adapter normalizes to canonical request.
3. Router resolves logical model alias and policy constraints.
4. If image input exists:
   - if selected model supports vision, run same-model multimodal path
   - else run split path: vision parser model -> merge -> generation model
5. Send to selected provider adapter.
6. If retriable failover class occurs:
   - pick next eligible provider from alias pool using fixed priority
   - retry with migration policy
7. Provider response normalized back into requested protocol format.

## Data Model (Phase 1)

Provider:
- id
- name (openai / anthropic / copilot / xiaomi / google / openrouter / deepseek)
- base_url
- auth_mode
- enabled
- health_state

ProviderModel:
- provider_id
- model_name
- enabled
- supports_vision
- supports_streaming
- cost_weight
- outbound_proxy_id (nullable)

LogicalModelAlias:
- alias_name
- task_type (chat, vision_parse, embedding)
- targets ordered list: provider/model pairs
- fallback_policy

RoutingPolicy:
- policy_id
- strategy (priority, weighted, latency-first, cost-first)
- failover_triggers
- retry_budget

Tenant:
- tenant_id
- name
- status

ApiKey:
- key_id
- tenant_id
- hashed_key
- quota_policy_id
- rate_limit_policy_id
- acl_policy_id

OutboundProxy:
- proxy_id
- name
- scheme
- host
- port
- auth_ref
- enabled

## Dashboard Scope (Phase 1)

- Provider switch: enabled/disabled hard switch
- Provider model availability config
- Alias mapping editor (logical model -> provider/model list)
- Failover trigger editor (balance, 429, 5xx, timeout, model offline, manual disable)
- Route strategy switcher (reliability/latency/cost)
- Tenant-scoped visibility and controls
- API key pool management with quota/rate-limit/ACL
- Per-model outbound proxy assignment
- Live status page: provider health, failover events, tenant-scoped stats

## Key Technical Choices

- Rust web stack: axum + tower
- Async runtime: tokio
- HTTP client: reqwest
- Serialization: serde
- Persistence: SQLite via sqlx (migration path to PostgreSQL later)
- Frontend: lightweight SPA served by backend (exact stack TBD)

## Risks and Mitigations

Risk: Protocol mismatch in edge cases
- Mitigation: golden test corpus for OpenAI and Claude payloads.

Risk: Fallback loops or cascading retries
- Mitigation: per-request retry budget and visited-provider set.

Risk: Image parsing latency inflation
- Mitigation: default direct multimodal route and split fallback only when needed.

Risk: Operator misconfiguration
- Mitigation: dashboard validation and dry-run route simulation.

Risk: Tenant data leakage
- Mitigation: strict tenant-scoped query filters and policy checks in every dashboard/API path.

Risk: Regional network failures for specific models
- Mitigation: per-model outbound proxy binding and health checks per egress path.
