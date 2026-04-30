import React, { useEffect, useState } from 'react'
import { AppLayout } from '../components/AppLayout'
import { apiClient } from '../api/client'
import { ProviderView, ProviderModel } from '../api/types'

export const ProviderMgmt: React.FC = () => {
  const [providers, setProviders] = useState<ProviderView[]>([])
  const [models, setModels] = useState<Record<string, ProviderModel[]>>({})
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  const load = async () => {
    try {
      const list = await apiClient.listProviders()
      setProviders(list)
    } catch (e: any) {
      setError(e.response?.data?.message || 'Failed to load providers')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    load()
  }, [])

  const toggleExpand = async (p: ProviderView) => {
    const next = new Set(expanded)
    if (next.has(p.id)) {
      next.delete(p.id)
    } else {
      next.add(p.id)
      if (!models[p.id]) {
        try {
          const m = await apiClient.listProviderModels(p.id)
          setModels((prev) => ({ ...prev, [p.id]: m }))
        } catch (e: any) {
          setError(e.response?.data?.message || 'Failed to load models')
        }
      }
    }
    setExpanded(next)
  }

  const onToggleProvider = async (p: ProviderView) => {
    try {
      await apiClient.setProviderEnabled(p.id, !p.enabled)
      await load()
    } catch (e: any) {
      setError(e.response?.data?.message || 'Failed to update provider')
    }
  }

  const onToggleModel = async (providerId: string, m: ProviderModel) => {
    try {
      await apiClient.setProviderModelEnabled(providerId, m.model_name, !m.enabled)
      const refreshed = await apiClient.listProviderModels(providerId)
      setModels((prev) => ({ ...prev, [providerId]: refreshed }))
    } catch (e: any) {
      setError(e.response?.data?.message || 'Failed to update model')
    }
  }

  return (
    <AppLayout>
      <h1 className="text-2xl font-bold text-gray-900 mb-6">Providers</h1>
      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4">
          {error}
        </div>
      )}
      {loading ? (
        <div className="text-gray-500">Loading…</div>
      ) : (
        <div className="bg-white rounded-lg shadow overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-gray-50 text-gray-600">
              <tr>
                <th className="px-6 py-3 text-left">Name</th>
                <th className="px-6 py-3 text-left">Display Name</th>
                <th className="px-6 py-3 text-left">Health</th>
                <th className="px-6 py-3 text-left">Enabled</th>
                <th className="px-6 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {providers.map((p) => (
                <React.Fragment key={p.id}>
                  <tr>
                    <td className="px-6 py-3 font-mono text-xs">{p.name}</td>
                    <td className="px-6 py-3">{p.display_name}</td>
                    <td className="px-6 py-3">
                      <HealthBadge state={p.health_state} />
                    </td>
                    <td className="px-6 py-3">
                      <Toggle on={!!p.enabled} onChange={() => onToggleProvider(p)} />
                    </td>
                    <td className="px-6 py-3 text-right">
                      <button
                        onClick={() => toggleExpand(p)}
                        className="text-indigo-600 hover:text-indigo-800 text-sm font-medium"
                      >
                        {expanded.has(p.id) ? 'Hide models' : 'Show models'}
                      </button>
                    </td>
                  </tr>
                  {expanded.has(p.id) && (
                    <tr>
                      <td colSpan={5} className="bg-gray-50 px-8 py-4">
                        {models[p.id]?.length ? (
                          <table className="w-full text-sm">
                            <thead className="text-gray-500">
                              <tr>
                                <th className="text-left py-1">Model</th>
                                <th className="text-left py-1">Capabilities</th>
                                <th className="text-left py-1 w-32">Enabled</th>
                              </tr>
                            </thead>
                            <tbody>
                              {models[p.id].map((m) => (
                                <tr key={m.id}>
                                  <td className="py-1 font-mono text-xs">{m.model_name}</td>
                                  <td className="py-1 text-xs text-gray-600">
                                    {m.supports_streaming ? 'stream ' : ''}
                                    {m.supports_tools ? 'tools ' : ''}
                                    {m.supports_vision ? 'vision ' : ''}
                                  </td>
                                  <td className="py-1">
                                    <Toggle
                                      on={!!m.enabled}
                                      onChange={() => onToggleModel(p.id, m)}
                                    />
                                  </td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        ) : (
                          <div className="text-gray-500 text-sm">No models</div>
                        )}
                      </td>
                    </tr>
                  )}
                </React.Fragment>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </AppLayout>
  )
}

const Toggle: React.FC<{ on: boolean; onChange: () => void }> = ({ on, onChange }) => (
  <button
    type="button"
    onClick={onChange}
    className={`relative inline-flex h-5 w-10 items-center rounded-full transition ${
      on ? 'bg-indigo-600' : 'bg-gray-300'
    }`}
  >
    <span
      className={`inline-block h-4 w-4 transform rounded-full bg-white transition ${
        on ? 'translate-x-5' : 'translate-x-1'
      }`}
    />
  </button>
)

const HealthBadge: React.FC<{ state: string }> = ({ state }) => {
  const cls =
    state === 'healthy'
      ? 'bg-green-100 text-green-800'
      : state === 'degraded'
      ? 'bg-yellow-100 text-yellow-800'
      : state === 'unhealthy'
      ? 'bg-red-100 text-red-800'
      : 'bg-gray-100 text-gray-700'
  return <span className={`px-2 py-0.5 rounded text-xs font-medium ${cls}`}>{state}</span>
}
