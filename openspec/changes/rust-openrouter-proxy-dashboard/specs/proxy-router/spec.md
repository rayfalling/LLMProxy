## ADDED Requirements

### Requirement: Full protocol compatibility with priority order
The system SHALL expose API endpoints compatible with Claude interfaces first, then OpenAI interfaces.

#### Scenario: Claude full-interface compatibility first
- WHEN phase 1 protocol work is implemented
- THEN Claude interface coverage is completed before OpenAI interface parity tasks are closed
- AND Claude-compatible request and response formats are preserved end to end

#### Scenario: OpenAI request compatibility
- WHEN a client sends an OpenAI-compatible request
- THEN the proxy accepts the payload without client-side schema rewrite
- AND returns OpenAI-compatible response shape

#### Scenario: Claude request compatibility
- WHEN a client sends a Claude-compatible request
- THEN the proxy accepts the payload without client-side schema rewrite
- AND returns Claude-compatible response shape

### Requirement: Multimodal split model pipeline
The system SHALL support image parsing and generation using separately configurable models.

#### Scenario: Shared model default
- WHEN request includes image input and selected model supports vision input
- THEN the proxy uses the same model for vision and text generation

#### Scenario: Split fallback for non-vision model
- WHEN request includes image input and selected model does not support vision input
- THEN the proxy invokes configured vision parsing model first
- AND sends parsed result plus text context to generation LLM

### Requirement: Provider and model control from dashboard
The system SHALL allow operators to enforce provider and model availability from dashboard controls.

#### Scenario: Provider force disable
- WHEN operator disables a provider in dashboard
- THEN routing engine excludes that provider from candidate set immediately

#### Scenario: Model allow-list update
- WHEN operator updates allowed model list for a provider
- THEN subsequent requests only route to currently allowed models

#### Scenario: Route policy granularity
- WHEN operator configures routing policy in dashboard
- THEN operator can configure at provider level, model level, and route-strategy level

### Requirement: Aggregated logical model with automatic migration
The system SHALL allow one logical model alias to map to equivalent models across providers with automatic migration.

#### Scenario: Primary provider unavailable
- WHEN primary provider for an alias returns a configured failover trigger error
- THEN router retries using next eligible provider/model target for same alias
- AND response remains in caller protocol format

#### Scenario: Fixed priority fallback order
- WHEN multiple provider/model targets exist for one logical alias
- THEN migration follows configured fixed priority order

#### Scenario: Trigger classes
- WHEN upstream returns balance insufficient, rate limit 429, 5xx, timeout, model offline, or provider is manually disabled
- THEN fallback migration is triggered

#### Scenario: Non-failover error class
- WHEN provider returns an error not in failover trigger classes
- THEN proxy returns the error to client without cross-provider migration

### Requirement: Multi-tenant key isolation and policy
The system SHALL enforce tenant isolation and key-scoped controls.

#### Scenario: Tenant-scoped visibility
- WHEN dashboard user accesses configuration or metrics with a tenant API key context
- THEN only that tenant's configuration and statistics are visible

#### Scenario: Key-scoped quota and rate limit
- WHEN a request is authenticated by downstream API key
- THEN quota, rate limit, and access control policies are applied for that key

#### Scenario: Upstream key pool mapping
- WHEN downstream API key is resolved
- THEN request uses mapped upstream independent key pool for provider calls

### Requirement: Model-level outbound proxy support
The system SHALL support outbound proxy configuration per model route.

#### Scenario: Model-specific egress proxy
- WHEN a model route has outbound proxy configured
- THEN upstream request for that model is sent through configured proxy

#### Scenario: No model proxy configured
- WHEN a model route has no outbound proxy configured
- THEN upstream request uses default egress path

### Requirement: Persistent configuration and statistics
The system SHALL persist configuration and statistics to database storage.

#### Scenario: Persisted control plane config
- WHEN provider/model/policy settings are updated
- THEN changes are stored durably in database

#### Scenario: Persisted tenant statistics
- WHEN requests are processed
- THEN tenant-scoped runtime statistics are stored durably in database

### Requirement: Switchable routing objectives
The system SHALL allow switching routing objective strategy.

#### Scenario: Strategy switch
- WHEN operator changes route objective in dashboard
- THEN router uses selected strategy for subsequent routing decisions
