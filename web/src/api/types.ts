// API request / response types — kept aligned with crates/dashboard endpoints.

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
  tenant_name: string
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

export interface ProviderView {
  id: string
  name: string
  display_name: string
  base_url: string
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
}

export interface ModelAlias {
  id: string
  alias_name: string
  route_strategy: string
  targets?: AliasTarget[]
}

export interface AliasTarget {
  id?: string
  provider_id: string
  model_name: string
  priority: number
  enabled: number
}

export interface KeyPoolMapping {
  api_key_id: string
  provider_key_ids: string[]
}

export interface VisionMapping {
  model_name: string
  vision_parser_alias: string
  generation_alias: string
}

export interface TenantStats {
  qps: number
  p50_latency_ms: number
  p95_latency_ms: number
  error_rate: number
  failover_count: number
}

export interface FailoverEvent {
  id: string
  tenant_id?: string
  alias_name: string
  original_provider: string
  failover_provider: string
  reason: string
  created_at: string
}

export interface ApiError {
  error: string
  message: string
}
