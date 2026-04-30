import axios, { AxiosInstance } from 'axios'
import { attachAuthHeader, handleUnauthorized } from './auth'
import {
  LoginRequest,
  LoginResponse,
  ProviderView,
  ProviderModel,
  TenantStats,
  FailoverEventView,
  AliasView,
  AliasTargetInput,
  KeyPoolMappingView,
  VisionMappingView,
  SetupRequest,
  SetupResponse,
  SetupStatus,
  MeResponse,
  CreateProviderRequest,
  ProviderKeyView,
  CreateProviderKeyRequest,
  ProviderKeyCreatedResponse,
  CreateProviderModelRequest,
  CreateAliasRequest,
  AliasCreatedResponse,
  ApiKeyView,
  CreateApiKeyRequest,
  ApiKeyCreatedResponse,
  CreateKeyPoolRequest,
  CreateVisionMappingRequest,
} from './types'

const API_BASE = '/api'

class ApiClient {
  private http: AxiosInstance

  constructor() {
    this.http = axios.create({ baseURL: API_BASE, timeout: 15000 })

    this.http.interceptors.request.use(attachAuthHeader)
    this.http.interceptors.response.use((r) => r, handleUnauthorized)
  }

  // setup / auth
  async getSetupStatus(): Promise<SetupStatus> {
    return (await this.http.get<SetupStatus>('/setup/status')).data
  }
  async setup(req: SetupRequest): Promise<SetupResponse> {
    return (await this.http.post<SetupResponse>('/setup', req)).data
  }
  async login(req: LoginRequest): Promise<LoginResponse> {
    return (await this.http.post<LoginResponse>('/auth/login', req)).data
  }
  async me(): Promise<MeResponse> {
    return (await this.http.get<MeResponse>('/me')).data
  }

  // providers
  async listProviders(): Promise<ProviderView[]> {
    return (await this.http.get<ProviderView[]>('/providers')).data
  }
  async createProvider(req: CreateProviderRequest): Promise<ProviderView> {
    return (await this.http.post<ProviderView>('/providers', req)).data
  }
  async deleteProvider(providerId: string): Promise<void> {
    await this.http.delete(`/providers/${providerId}`)
  }
  async setProviderEnabled(providerId: string, enabled: boolean): Promise<void> {
    await this.http.put(`/providers/${providerId}/enabled`, { enabled })
  }
  async listProviderModels(providerId: string): Promise<ProviderModel[]> {
    return (await this.http.get<ProviderModel[]>(`/providers/${providerId}/models`)).data
  }
  async createProviderModel(
    providerId: string,
    req: CreateProviderModelRequest,
  ): Promise<ProviderModel> {
    return (await this.http.post<ProviderModel>(`/providers/${providerId}/models`, req)).data
  }
  async deleteProviderModel(providerId: string, modelName: string): Promise<void> {
    await this.http.delete(`/providers/${providerId}/models/${encodeURIComponent(modelName)}`)
  }
  async setProviderModelEnabled(
    providerId: string,
    modelName: string,
    enabled: boolean,
  ): Promise<void> {
    await this.http.put(
      `/providers/${providerId}/models/${encodeURIComponent(modelName)}/enabled`,
      { enabled },
    )
  }
  async listProviderKeys(providerId: string): Promise<ProviderKeyView[]> {
    return (await this.http.get<ProviderKeyView[]>(`/providers/${providerId}/keys`)).data
  }
  async createProviderKey(
    providerId: string,
    req: CreateProviderKeyRequest,
  ): Promise<ProviderKeyCreatedResponse> {
    return (
      await this.http.post<ProviderKeyCreatedResponse>(`/providers/${providerId}/keys`, req)
    ).data
  }
  async deleteProviderKey(providerId: string, keyId: string): Promise<void> {
    await this.http.delete(`/providers/${providerId}/keys/${keyId}`)
  }

  // aliases
  async listAliases(): Promise<AliasView[]> {
    return (await this.http.get<AliasView[]>('/aliases')).data
  }
  async createAlias(req: CreateAliasRequest): Promise<AliasCreatedResponse> {
    return (await this.http.post<AliasCreatedResponse>('/aliases', req)).data
  }
  async deleteAlias(aliasName: string): Promise<void> {
    await this.http.delete(`/aliases/${encodeURIComponent(aliasName)}`)
  }
  async updateAliasStrategy(aliasName: string, routeStrategy: string): Promise<void> {
    await this.http.put(`/aliases/${encodeURIComponent(aliasName)}/strategy`, {
      route_strategy: routeStrategy,
    })
  }
  async updateAliasTargets(aliasName: string, targets: AliasTargetInput[]): Promise<void> {
    await this.http.put(`/aliases/${encodeURIComponent(aliasName)}/targets`, { targets })
  }

  // tenant api keys
  async listApiKeys(): Promise<ApiKeyView[]> {
    return (await this.http.get<ApiKeyView[]>('/api-keys')).data
  }
  async createApiKey(req: CreateApiKeyRequest): Promise<ApiKeyCreatedResponse> {
    return (await this.http.post<ApiKeyCreatedResponse>('/api-keys', req)).data
  }
  async deleteApiKey(apiKeyId: string): Promise<void> {
    await this.http.delete(`/api-keys/${apiKeyId}`)
  }

  // key pools
  async listKeyPoolMappings(): Promise<KeyPoolMappingView[]> {
    return (await this.http.get<KeyPoolMappingView[]>('/key-pools')).data
  }
  async createKeyPool(req: CreateKeyPoolRequest): Promise<void> {
    await this.http.post('/key-pools', req)
  }
  async deleteKeyPool(apiKeyId: string, providerId: string): Promise<void> {
    await this.http.delete(`/key-pools/${apiKeyId}/${providerId}`)
  }
  async updateKeyPoolMapping(apiKeyId: string, providerKeyIds: string[]): Promise<void> {
    await this.http.put(`/key-pools/${apiKeyId}`, { provider_key_ids: providerKeyIds })
  }

  // vision mappings
  async listVisionMappings(): Promise<VisionMappingView[]> {
    return (await this.http.get<VisionMappingView[]>('/vision-mappings')).data
  }
  async createVisionMapping(req: CreateVisionMappingRequest): Promise<VisionMappingView> {
    return (await this.http.post<VisionMappingView>('/vision-mappings', req)).data
  }
  async deleteVisionMapping(modelName: string): Promise<void> {
    await this.http.delete(`/vision-mappings/${encodeURIComponent(modelName)}`)
  }
  async updateVisionMapping(
    modelName: string,
    visionParserAlias: string | null,
    generationAlias: string | null,
  ): Promise<void> {
    await this.http.put(`/vision-mappings/${encodeURIComponent(modelName)}`, {
      vision_parser_alias: visionParserAlias,
      generation_alias: generationAlias,
    })
  }

  // stats
  async getTenantStats(): Promise<TenantStats> {
    return (await this.http.get<TenantStats>('/stats')).data
  }
  async listFailoverEvents(limit = 20): Promise<FailoverEventView[]> {
    return (
      await this.http.get<FailoverEventView[]>('/events/failovers', {
        params: { limit },
      })
    ).data
  }
}

export const apiClient = new ApiClient()
