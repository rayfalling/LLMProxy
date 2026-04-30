import axios, { AxiosInstance } from 'axios'
import {
  LoginRequest,
  LoginResponse,
  ProviderView,
  ProviderModel,
  TenantStats,
  FailoverEvent,
  ModelAlias,
  AliasTarget,
  KeyPoolMapping,
  VisionMapping,
  SetupRequest,
  SetupResponse,
  SetupStatus,
  MeResponse,
} from './types'

const API_BASE = '/api'

class ApiClient {
  private http: AxiosInstance

  constructor() {
    this.http = axios.create({
      baseURL: API_BASE,
      timeout: 15000,
    })

    this.http.interceptors.request.use((config) => {
      const token = localStorage.getItem('jwt_token')
      if (token) {
        config.headers.Authorization = `Bearer ${token}`
      }
      return config
    })

    this.http.interceptors.response.use(
      (response) => response,
      (error) => {
        if (error.response?.status === 401) {
          localStorage.removeItem('jwt_token')
          if (
            window.location.pathname !== '/' &&
            window.location.pathname !== '/setup'
          ) {
            window.location.href = '/'
          }
        }
        return Promise.reject(error)
      },
    )
  }

  // ── setup / auth ────────────────────────────────────────────────────────
  async getSetupStatus(): Promise<SetupStatus> {
    const r = await this.http.get<SetupStatus>('/setup/status')
    return r.data
  }

  async setup(req: SetupRequest): Promise<SetupResponse> {
    const r = await this.http.post<SetupResponse>('/setup', req)
    return r.data
  }

  async login(req: LoginRequest): Promise<LoginResponse> {
    const r = await this.http.post<LoginResponse>('/auth/login', req)
    return r.data
  }

  async me(): Promise<MeResponse> {
    const r = await this.http.get<MeResponse>('/me')
    return r.data
  }

  // ── providers ───────────────────────────────────────────────────────────
  async listProviders(): Promise<ProviderView[]> {
    const r = await this.http.get<ProviderView[]>('/providers')
    return r.data
  }

  async setProviderEnabled(providerId: string, enabled: boolean) {
    await this.http.put(`/providers/${providerId}/enabled`, { enabled })
  }

  async listProviderModels(providerId: string): Promise<ProviderModel[]> {
    const r = await this.http.get<ProviderModel[]>(`/providers/${providerId}/models`)
    return r.data
  }

  async setProviderModelEnabled(
    providerId: string,
    modelName: string,
    enabled: boolean,
  ) {
    await this.http.put(
      `/providers/${providerId}/models/${encodeURIComponent(modelName)}/enabled`,
      { enabled },
    )
  }

  // ── aliases ─────────────────────────────────────────────────────────────
  async listAliases(): Promise<ModelAlias[]> {
    const r = await this.http.get<ModelAlias[]>('/aliases')
    return r.data
  }

  async updateAliasStrategy(aliasName: string, routeStrategy: string) {
    await this.http.put(`/aliases/${encodeURIComponent(aliasName)}/strategy`, {
      route_strategy: routeStrategy,
    })
  }

  async updateAliasTargets(aliasName: string, targets: AliasTarget[]) {
    await this.http.put(`/aliases/${encodeURIComponent(aliasName)}/targets`, {
      targets,
    })
  }

  // ── key pools ───────────────────────────────────────────────────────────
  async listKeyPoolMappings(): Promise<KeyPoolMapping[]> {
    const r = await this.http.get<KeyPoolMapping[]>('/key-pools')
    return r.data
  }

  async updateKeyPoolMapping(apiKeyId: string, providerKeyIds: string[]) {
    await this.http.put(`/key-pools/${apiKeyId}`, { provider_key_ids: providerKeyIds })
  }

  // ── vision mappings ─────────────────────────────────────────────────────
  async listVisionMappings(): Promise<VisionMapping[]> {
    const r = await this.http.get<VisionMapping[]>('/vision-mappings')
    return r.data
  }

  async updateVisionMapping(
    modelName: string,
    visionParserAlias: string,
    generationAlias: string,
  ) {
    await this.http.put(`/vision-mappings/${encodeURIComponent(modelName)}`, {
      vision_parser_alias: visionParserAlias,
      generation_alias: generationAlias,
    })
  }

  // ── stats ───────────────────────────────────────────────────────────────
  async getTenantStats(): Promise<TenantStats> {
    const r = await this.http.get<TenantStats>('/stats')
    return r.data
  }

  async listFailoverEvents(limit = 20): Promise<FailoverEvent[]> {
    const r = await this.http.get<FailoverEvent[]>('/events/failovers', {
      params: { limit },
    })
    return r.data
  }
}

export const apiClient = new ApiClient()
