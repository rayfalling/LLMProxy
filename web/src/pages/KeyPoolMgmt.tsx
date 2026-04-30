import React, { useEffect, useMemo, useState } from 'react'
import { AppLayout } from '../components/AppLayout'
import { ConfirmDeleteDialog } from '../components/ConfirmDeleteDialog'
import { apiClient } from '../api/client'
import {
  ApiKeyView,
  KeyPoolMappingView,
  ProviderKeyView,
  ProviderView,
} from '../api/types'

interface PendingDelete {
  apiKeyId: string
  providerId: string
  providerName: string
  apiKeyLabel: string
}

export const KeyPoolMgmt: React.FC = () => {
  const [mappings, setMappings] = useState<KeyPoolMappingView[]>([])
  const [apiKeys, setApiKeys] = useState<ApiKeyView[]>([])
  const [providers, setProviders] = useState<ProviderView[]>([])
  const [providerKeys, setProviderKeys] = useState<Record<string, ProviderKeyView[]>>({})
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [showCreate, setShowCreate] = useState(false)
  const [pendingDelete, setPendingDelete] = useState<PendingDelete | null>(null)

  const load = async () => {
    try {
      const [m, a, p] = await Promise.all([
        apiClient.listKeyPoolMappings(),
        apiClient.listApiKeys(),
        apiClient.listProviders(),
      ])
      setMappings(m)
      setApiKeys(a)
      setProviders(p)
      const pkMap: Record<string, ProviderKeyView[]> = {}
      await Promise.all(
        p.map(async (prov) => {
          pkMap[prov.id] = await apiClient.listProviderKeys(prov.id)
        }),
      )
      setProviderKeys(pkMap)
    } catch (e: any) {
      setError(e.response?.data?.message || 'Failed to load key pools')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    load()
  }, [])

  const grouped = useMemo(() => {
    type Row = { apiKeyId: string; providerId: string; providerKeyIds: string[] }
    const lookup = new Map<string, string>()
    Object.entries(providerKeys).forEach(([provId, list]) => {
      list.forEach((k) => lookup.set(k.id, provId))
    })
    const rows = new Map<string, Row>()
    mappings.forEach((m) => {
      const provId = lookup.get(m.provider_key_id)
      if (!provId) return
      const k = `${m.api_key_id}__${provId}`
      const r = rows.get(k) ?? {
        apiKeyId: m.api_key_id,
        providerId: provId,
        providerKeyIds: [],
      }
      r.providerKeyIds.push(m.provider_key_id)
      rows.set(k, r)
    })
    return Array.from(rows.values())
  }, [mappings, providerKeys])

  const apiKeyLabel = (id: string): string => {
    const k = apiKeys.find((x) => x.id === id)
    return k ? `${k.name ?? '(unnamed)'} — ${k.prefix}…` : id
  }
  const providerLabel = (id: string): string => providers.find((p) => p.id === id)?.name ?? id

  return (
    <AppLayout>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900">Key pools</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="px-3 py-1.5 text-sm bg-indigo-600 text-white rounded-md hover:bg-indigo-700 transition"
        >
          + Add mapping
        </button>
      </div>
      <p className="text-sm text-gray-500 mb-4">
        A mapping pins one of your tenant API keys to a subset of provider keys for a given
        provider. Without an explicit mapping, the proxy may use any enabled provider key.
      </p>
      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4 flex justify-between">
          <span>{error}</span>
          <button onClick={() => setError(null)} className="ml-2 text-sm underline">
            dismiss
          </button>
        </div>
      )}
      {loading ? (
        <div className="text-gray-500">Loading…</div>
      ) : grouped.length === 0 ? (
        <div className="bg-white border-2 border-dashed border-gray-200 rounded-lg p-12 text-center text-gray-500">
          No key-pool mappings yet. Click <strong>Add mapping</strong> to create one.
        </div>
      ) : (
        <div className="bg-white rounded-lg shadow overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-gray-50 text-gray-600">
              <tr>
                <th className="px-6 py-3 text-left">Inbound API key</th>
                <th className="px-6 py-3 text-left">Provider</th>
                <th className="px-6 py-3 text-left">Allowed provider keys</th>
                <th className="px-6 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {grouped.map((r) => (
                <tr key={`${r.apiKeyId}__${r.providerId}`}>
                  <td className="px-6 py-3">{apiKeyLabel(r.apiKeyId)}</td>
                  <td className="px-6 py-3 font-mono text-xs">{providerLabel(r.providerId)}</td>
                  <td className="px-6 py-3">
                    <div className="flex flex-wrap gap-1">
                      {r.providerKeyIds.map((id) => {
                        const k = providerKeys[r.providerId]?.find((x) => x.id === id)
                        return (
                          <span
                            key={id}
                            className="bg-indigo-50 text-indigo-700 text-xs font-mono px-2 py-0.5 rounded"
                          >
                            {k?.key_preview ?? id}
                          </span>
                        )
                      })}
                    </div>
                  </td>
                  <td className="px-6 py-3 text-right">
                    <button
                      onClick={() =>
                        setPendingDelete({
                          apiKeyId: r.apiKeyId,
                          providerId: r.providerId,
                          providerName: providerLabel(r.providerId),
                          apiKeyLabel: apiKeyLabel(r.apiKeyId),
                        })
                      }
                      className="text-red-600 hover:text-red-800 text-sm font-medium"
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {showCreate && (
        <CreateKeyPoolModal
          apiKeys={apiKeys}
          providers={providers}
          providerKeys={providerKeys}
          onCancel={() => setShowCreate(false)}
          onCreated={async () => {
            setShowCreate(false)
            await load()
          }}
        />
      )}
      {pendingDelete && (
        <ConfirmDeleteDialog
          title="Delete mapping"
          message={
            <span>
              Remove the binding from <strong>{pendingDelete.apiKeyLabel}</strong> to provider{' '}
              <code>{pendingDelete.providerName}</code>?
            </span>
          }
          confirmText="Delete"
          onCancel={() => setPendingDelete(null)}
          onConfirm={async () => {
            await apiClient.deleteKeyPool(pendingDelete.apiKeyId, pendingDelete.providerId)
            setPendingDelete(null)
            await load()
          }}
        />
      )}
    </AppLayout>
  )
}

interface CreateProps {
  apiKeys: ApiKeyView[]
  providers: ProviderView[]
  providerKeys: Record<string, ProviderKeyView[]>
  onCancel: () => void
  onCreated: () => Promise<void>
}

const CreateKeyPoolModal: React.FC<CreateProps> = ({
  apiKeys,
  providers,
  providerKeys,
  onCancel,
  onCreated,
}) => {
  const activeKeys = apiKeys.filter((k) => k.status === 'active')
  const [apiKeyId, setApiKeyId] = useState(activeKeys[0]?.id ?? '')
  const [providerId, setProviderId] = useState(providers[0]?.id ?? '')
  const [allowed, setAllowed] = useState<Set<string>>(new Set())
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const availableKeys = providerKeys[providerId] ?? []

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!apiKeyId || !providerId || allowed.size === 0) {
      setError('Pick an API key, provider, and at least one provider key.')
      return
    }
    setBusy(true)
    setError(null)
    try {
      await apiClient.createKeyPool({
        api_key_id: apiKeyId,
        provider_id: providerId,
        allowed_provider_key_ids: Array.from(allowed),
      })
    } catch (err: any) {
      setError(err?.response?.data?.message || 'Request failed')
      setBusy(false)
      return
    }
    setBusy(false)
    await onCreated()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <form onSubmit={submit} className="bg-white rounded-lg shadow-xl w-full max-w-md p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Add key-pool mapping</h2>
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded mb-3 text-sm">
            {error}
          </div>
        )}
        <label className="block text-sm mb-3">
          <span className="text-gray-700">Inbound API key</span>
          <select
            value={apiKeyId}
            onChange={(e) => setApiKeyId(e.target.value)}
            className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2 text-sm"
          >
            {activeKeys.length === 0 && (
              <option value="">(no active keys — create one first)</option>
            )}
            {activeKeys.map((k) => (
              <option key={k.id} value={k.id}>
                {k.name ?? '(unnamed)'} — {k.prefix}…
              </option>
            ))}
          </select>
        </label>
        <label className="block text-sm mb-3">
          <span className="text-gray-700">Provider</span>
          <select
            value={providerId}
            onChange={(e) => {
              setProviderId(e.target.value)
              setAllowed(new Set())
            }}
            className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2 text-sm"
          >
            {providers.length === 0 && (
              <option value="">(no providers — register one first)</option>
            )}
            {providers.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        <fieldset className="mb-4">
          <legend className="text-sm text-gray-700 mb-1">Allowed provider keys</legend>
          {availableKeys.length === 0 ? (
            <div className="text-xs text-gray-500">No keys registered for this provider.</div>
          ) : (
            <div className="space-y-1 max-h-40 overflow-auto border border-gray-200 rounded p-2">
              {availableKeys.map((k) => (
                <label key={k.id} className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={allowed.has(k.id)}
                    onChange={(e) =>
                      setAllowed((prev) => {
                        const next = new Set(prev)
                        if (e.target.checked) next.add(k.id)
                        else next.delete(k.id)
                        return next
                      })
                    }
                  />
                  <span className="font-mono text-xs">{k.key_preview}</span>
                  {k.label && <span className="text-gray-500 text-xs">({k.label})</span>}
                </label>
              ))}
            </div>
          )}
        </fieldset>
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-100 rounded-md"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={busy}
            className="px-4 py-1.5 text-sm bg-indigo-600 text-white rounded-md hover:bg-indigo-700 disabled:opacity-50"
          >
            {busy ? 'Submitting…' : 'Create'}
          </button>
        </div>
      </form>
    </div>
  )
}
