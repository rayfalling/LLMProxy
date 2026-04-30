## 1. Foundation

- [x] 1.1 Initialize Rust workspace structure for proxy, dashboard API, and shared core.
- [x] 1.2 Define canonical request/response schema and error taxonomy.
- [x] 1.3 Set up SQLite via sqlx with migration baseline.
- [x] 1.4 Implement Claude full-interface protocol adapters (stable + beta) and contract tests.
- [x] 1.5 Implement OpenAI full-interface protocol adapters (chat/responses/images/audio/realtime) and contract tests.

## 2. Provider and Routing

- [x] 2.1 Create provider adapter trait and base execution context.
- [x] 2.2 Implement OpenAI adapter (chat/responses/images/audio/realtime).
- [x] 2.3 Implement Anthropic adapter (full API including beta headers).
- [x] 2.4 Implement Copilot adapter (Azure OpenAI endpoint).
- [x] 2.5 Implement Xiaomi adapter (Mi AI API).
- [x] 2.6 Implement Google adapter (Gemini API).
- [x] 2.7 Implement OpenRouter adapter.
- [x] 2.8 Implement DeepSeek official adapter.
- [x] 2.9 Implement logical model alias registry.
- [x] 2.10 Implement fixed-priority failover engine and retry budget.
- [x] 2.11 Implement failover trigger mapping for balance, 429, 5xx, timeout, model offline, and manual disable.
- [x] 2.12 Implement switchable route objective policies (reliability/latency/cost).

## 3. Multimodal Image Parsing

- [x] 3.1 Add image input normalization in canonical schema.
- [x] 3.2 Implement default same-model multimodal execution when model supports vision.
- [x] 3.3 Implement split fallback pipeline: vision parser model + generation model.
- [ ] 3.4 Add dashboard fields for vision/generation model mapping.
- [x] 3.5 Add per-model outbound proxy configuration and runtime binding.

## 4. Dashboard and Control Plane

- [x] 4.1 Implement tenant admin JWT authentication (no SSO).
- [x] 4.2 Implement dashboard backend APIs for tenant-scoped providers/models/policies.
- [x] 4.3 Implement provider enable/disable force switch.
- [x] 4.4 Implement model allow-list configuration per provider.
- [x] 4.5 Implement alias mapping and migration policy editor.
- [x] 4.6 Implement route strategy switch configuration.
- [x] 4.7 Implement API key pool mapping to upstream independent key pools.
- [x] 4.8 Implement status views for health, failover events, and tenant-scoped stats.

## 5. Observability and Safety

- [ ] 5.1 Add structured request logs with sensitive field redaction.
- [ ] 5.2 Add tenant-scoped metrics: QPS, p50/p95 latency, error rate, failover count.
- [ ] 5.3 Add API key mapping and request-level tenant authorization checks.
- [ ] 5.4 Add quota and rate limiting per downstream key.
- [ ] 5.5 Add model-level access control policy per key.
- [ ] 5.6 Add relational DB persistence for config and statistics.

## 6. Validation

- [ ] 6.1 Contract tests for Claude full-interface compatibility.
- [ ] 6.2 Contract tests for OpenAI full-interface compatibility.
- [ ] 6.3 End-to-end tests for default same-model multimodal and split fallback modes.
- [ ] 6.4 End-to-end tests for provider outage and fixed-priority automatic migration.
- [ ] 6.5 End-to-end tests for model-level outbound proxy routing.
- [ ] 6.6 Dashboard integration tests for tenant/provider/model/route controls.

## 7. Release Readiness

- [ ] 7.1 Produce deployment profile: single binary first.
- [ ] 7.2 Add environment and config documentation.
- [ ] 7.3 Add runbook for failover incident operations and proxy-region issues.
- [ ] 7.4 Mark change ready after open questions are resolved.
