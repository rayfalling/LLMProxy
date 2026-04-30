// API request / response types — mirror crates/dashboard/src/handlers.rs.

export interface SetupStatus {
  initialized: boolean
}

export interface SetupRequest {
  tenant_name: string
  username: string
  password: string
  password_confirm: string
}

export interface SetupResponse {
  success: boolean
  tenant_id: string
  admin_id: string
}

export interface LoginRequest {
  username: string
  password: string
}

export interface LoginResponse {
  token: string
  token_type: string
  expires_in: number
  tenant_id: string
  username: string
}

export interface MeResponse {
  tenant_id: string
  username: string
}

// providers
export interface ProviderView {
  id: string
  name: string
  display_name: string
  enabled: number
  health_state: string
}

export interface ProviderModel {
  id: string
  provider_id: string
  model_name: string
  enabled: number
  supports_vision: number
  supports_streaming: number
  supports_tools: number
  outbound_proxy_id: string | null
}

// aliases
export interface AliasView {
  id: string
  alias_name: string
  description: string | null
  route_strategy: string
}

export interface AliasTargetInput {
  provider_id: string
  model_name: string
  priority: number
  enabled: boolean
}

// key pools (flat list of (api_key_id, provider_key_id) pairs)
export interface KeyPoolMappingView {
  api_key_id: string
  provider_key_id: string
}

// vision mappings
export interface VisionMappingView {
  model_name: string
  vision_parser_alias: string | null
  generation_alias: string | null
}

// stats
export interface TenantStats {
  total_requests: number
  total_input_tokens: number
  total_output_tokens: number
  avg_latency_ms: number
  qps_last_hour: number
  p50_latency_ms_last_hour: number
  p95_latency_ms_last_hour: number
  error_rate_last_hour: number
  failover_count_last_hour: number
}

export interface FailoverEventView {
  request_id: string
  model_alias: string
  provider_id: string | null
  provider_model: string | null
  failover_count: number
  error_code: string | null
  created_at: string
}

export interface ApiError {
  error: string
  message: string
}

export interface ActionResponse {
  ok: boolean
}

// ── create/delete request payloads (webui-resource-crud-and-tenantless-login) ──

export interface CreateProviderRequest {
  name: string
  display_name: string
  base_url: string
  auth_mode?: string
  auth_header?: string | null
}

export interface ProviderKeyView {
  id: string
  provider_id: string
  label: string | null
  enabled: number
  priority: number
  key_preview: string
}

export interface CreateProviderKeyRequest {
  label?: string | null
  plaintext_key: string
  priority?: number
}

export interface ProviderKeyCreatedResponse {
  id: string
  provider_id: string
  label: string | null
  priority: number
  enabled: number
}

export interface CreateProviderModelRequest {
  model_name: string
  supports_vision?: boolean
  supports_streaming?: boolean
  supports_tools?: boolean
  context_window?: number | null
  max_output_tokens?: number | null
}

export interface CreateAliasRequest {
  alias_name: string
  description?: string | null
  route_strategy?: string
  targets?: AliasTargetInput[]
}

export interface AliasCreatedResponse {
  id: string
  alias_name: string
  description: string | null
  route_strategy: string
  targets_count: number
}

export interface ApiKeyView {
  id: string
  name: string | null
  prefix: string
  status: string
  quota_rpm: number | null
  quota_tpm: number | null
  quota_daily_req: number | null
  created_at: string
  updated_at: string
}

export interface CreateApiKeyRequest {
  name?: string | null
  quota_rpm?: number | null
  quota_tpm?: number | null
  quota_daily_req?: number | null
}

export interface ApiKeyCreatedResponse {
  id: string
  name: string | null
  plaintext_key: string
  prefix: string
  status: string
  created_at: string
}

export interface CreateKeyPoolRequest {
  api_key_id: string
  provider_id: string
  allowed_provider_key_ids: string[]
}

export interface CreateVisionMappingRequest {
  model_name: string
  vision_parser_alias?: string | null
  generation_alias?: string | null
}
