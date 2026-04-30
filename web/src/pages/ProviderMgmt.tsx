import React, { useEffect, useState } from 'react'
import { AppLayout } from '../components/AppLayout'
import { ResourceCreateModal, ModalField } from '../components/ResourceCreateModal'
import { ConfirmDeleteDialog } from '../components/ConfirmDeleteDialog'
import { apiClient } from '../api/client'
import { ProviderView, ProviderModel, ProviderKeyView } from '../api/types'

type Modal =
  | { kind: 'add-provider' }
  | { kind: 'delete-provider'; provider: ProviderView }
  | { kind: 'add-model'; provider: ProviderView }
  | { kind: 'delete-model'; provider: ProviderView; model: ProviderModel }
  | { kind: 'add-key'; provider: ProviderView }
  | { kind: 'delete-key'; provider: ProviderView; key: ProviderKeyView }

export const ProviderMgmt: React.FC = () => {
  const [providers, setProviders] = useState<ProviderView[]>([])
  const [models, setModels] = useState<Record<string, ProviderModel[]>>({})
  const [keys, setKeys] = useState<Record<string, ProviderKeyView[]>>({})
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [modal, setModal] = useState<Modal | null>(null)

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

  const refreshChildren = async (pid: string) => {
    const [m, k] = await Promise.all([
      apiClient.listProviderModels(pid),
      apiClient.listProviderKeys(pid),
    ])
    setModels((prev) => ({ ...prev, [pid]: m }))
    setKeys((prev) => ({ ...prev, [pid]: k }))
  }

  const toggleExpand = async (p: ProviderView) => {
    const next = new Set(expanded)
    if (next.has(p.id)) {
      next.delete(p.id)
    } else {
      next.add(p.id)
      if (!models[p.id] || !keys[p.id]) {
        try {
          await refreshChildren(p.id)
        } catch (e: any) {
          setError(e.response?.data?.message || 'Failed to load provider details')
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
      await refreshChildren(providerId)
    } catch (e: any) {
      setError(e.response?.data?.message || 'Failed to update model')
    }
  }

  const providerFields: ModalField[] = [
    {
      name: 'name',
      label: 'Identifier',
      required: true,
      placeholder: 'e.g. openai',
      helpText: 'lowercase letters, digits, _ or - (2-32 chars)',
    },
    { name: 'display_name', label: 'Display name', required: true, placeholder: 'OpenAI' },
    {
      name: 'base_url',
      label: 'Base URL',
      type: 'url',
      required: true,
      placeholder: 'https://api.openai.com',
    },
    {
      name: 'auth_mode',
      label: 'Auth mode',
      type: 'select',
      required: true,
      defaultValue: 'bearer',
      options: [
        { value: 'bearer', label: 'Bearer token' },
        { value: 'api-key-header', label: 'Custom API-key header' },
      ],
    },
    {
      name: 'auth_header',
      label: 'Auth header name (api-key-header only)',
      placeholder: 'e.g. x-api-key',
    },
  ]

  const keyFields: ModalField[] = [
    { name: 'label', label: 'Label (optional)', placeholder: 'e.g. main' },
    { name: 'plaintext_key', label: 'API key', type: 'password', required: true },
    { name: 'priority', label: 'Priority', type: 'number', defaultValue: 0 },
  ]

  const modelFields: ModalField[] = [
    {
      name: 'model_name',
      label: 'Model name',
      required: true,
      placeholder: 'e.g. gpt-4o-2024-11-20',
    },
    { name: 'context_window', label: 'Context window (tokens)', type: 'number' },
    { name: 'max_output_tokens', label: 'Max output tokens', type: 'number' },
    {
      name: 'supports_vision',
      label: 'Supports vision',
      type: 'select',
      defaultValue: 'false',
      options: [
        { value: 'false', label: 'No' },
        { value: 'true', label: 'Yes' },
      ],
    },
    {
      name: 'supports_streaming',
      label: 'Supports streaming',
      type: 'select',
      defaultValue: 'true',
      options: [
        { value: 'true', label: 'Yes' },
        { value: 'false', label: 'No' },
      ],
    },
    {
      name: 'supports_tools',
      label: 'Supports tools',
      type: 'select',
      defaultValue: 'true',
      options: [
        { value: 'true', label: 'Yes' },
        { value: 'false', label: 'No' },
      ],
    },
  ]

  return (
    <AppLayout>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900">Providers</h1>
        <button
          onClick={() => setModal({ kind: 'add-provider' })}
          className="px-3 py-1.5 text-sm bg-indigo-600 text-white rounded-md hover:bg-indigo-700 transition"
        >
          + Add provider
        </button>
      </div>
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
      ) : providers.length === 0 ? (
        <div className="bg-white border-2 border-dashed border-gray-200 rounded-lg p-12 text-center text-gray-500">
          No providers yet. Click <strong>Add provider</strong> to register one.
        </div>
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
                        className="text-indigo-600 hover:text-indigo-800 text-sm font-medium mr-4"
                      >
                        {expanded.has(p.id) ? 'Hide' : 'Manage'}
                      </button>
                      <button
                        onClick={() => setModal({ kind: 'delete-provider', provider: p })}
                        className="text-red-600 hover:text-red-800 text-sm font-medium"
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                  {expanded.has(p.id) && (
                    <tr>
                      <td colSpan={5} className="bg-gray-50 px-8 py-4">
                        <ProviderDetails
                          models={models[p.id] ?? []}
                          keys={keys[p.id] ?? []}
                          onAddModel={() => setModal({ kind: 'add-model', provider: p })}
                          onDeleteModel={(m) =>
                            setModal({ kind: 'delete-model', provider: p, model: m })
                          }
                          onToggleModel={(m) => onToggleModel(p.id, m)}
                          onAddKey={() => setModal({ kind: 'add-key', provider: p })}
                          onDeleteKey={(k) =>
                            setModal({ kind: 'delete-key', provider: p, key: k })
                          }
                        />
                      </td>
                    </tr>
                  )}
                </React.Fragment>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {modal?.kind === 'add-provider' && (
        <ResourceCreateModal
          title="Add provider"
          fields={providerFields}
          onCancel={() => setModal(null)}
          onSubmit={async (v) => {
            await apiClient.createProvider({
              name: v.name,
              display_name: v.display_name,
              base_url: v.base_url,
              auth_mode: v.auth_mode || undefined,
              auth_header: v.auth_header || undefined,
            })
            setModal(null)
            await load()
          }}
        />
      )}
      {modal?.kind === 'delete-provider' && (
        <ConfirmDeleteDialog
          title="Delete provider"
          message={
            <span>
              Permanently delete provider <code>{modal.provider.name}</code>? Its keys and models
              will be removed too. Aliases that target this provider will block the delete.
            </span>
          }
          confirmText="Delete"
          typedConfirmation={modal.provider.name}
          onCancel={() => setModal(null)}
          onConfirm={async () => {
            await apiClient.deleteProvider(modal.provider.id)
            setModal(null)
            await load()
          }}
        />
      )}
      {modal?.kind === 'add-model' && (
        <ResourceCreateModal
          title={`Add model to ${modal.provider.name}`}
          fields={modelFields}
          onCancel={() => setModal(null)}
          onSubmit={async (v) => {
            await apiClient.createProviderModel(modal.provider.id, {
              model_name: v.model_name,
              supports_vision: v.supports_vision === 'true',
              supports_streaming: v.supports_streaming === 'true',
              supports_tools: v.supports_tools === 'true',
              context_window: v.context_window ? Number(v.context_window) : null,
              max_output_tokens: v.max_output_tokens ? Number(v.max_output_tokens) : null,
            })
            setModal(null)
            await refreshChildren(modal.provider.id)
          }}
        />
      )}
      {modal?.kind === 'delete-model' && (
        <ConfirmDeleteDialog
          title="Delete model"
          message={
            <span>
              Remove <code>{modal.model.model_name}</code> from{' '}
              <code>{modal.provider.name}</code>?
            </span>
          }
          confirmText="Delete"
          onCancel={() => setModal(null)}
          onConfirm={async () => {
            await apiClient.deleteProviderModel(modal.provider.id, modal.model.model_name)
            setModal(null)
            await refreshChildren(modal.provider.id)
          }}
        />
      )}
      {modal?.kind === 'add-key' && (
        <ResourceCreateModal
          title={`Add API key for ${modal.provider.name}`}
          fields={keyFields}
          onCancel={() => setModal(null)}
          onSubmit={async (v) => {
            await apiClient.createProviderKey(modal.provider.id, {
              label: v.label || null,
              plaintext_key: v.plaintext_key,
              priority: v.priority ? Number(v.priority) : 0,
            })
            setModal(null)
            await refreshChildren(modal.provider.id)
          }}
        />
      )}
      {modal?.kind === 'delete-key' && (
        <ConfirmDeleteDialog
          title="Delete provider key"
          message={
            <span>
              Remove key <code>{modal.key.key_preview}</code>
              {modal.key.label && <> ({modal.key.label})</>}? Tenant key-pool mappings that
              reference this key will block the delete.
            </span>
          }
          confirmText="Delete"
          onCancel={() => setModal(null)}
          onConfirm={async () => {
            await apiClient.deleteProviderKey(modal.provider.id, modal.key.id)
            setModal(null)
            await refreshChildren(modal.provider.id)
          }}
        />
      )}
    </AppLayout>
  )
}

const ProviderDetails: React.FC<{
  models: ProviderModel[]
  keys: ProviderKeyView[]
  onAddModel: () => void
  onDeleteModel: (m: ProviderModel) => void
  onToggleModel: (m: ProviderModel) => void
  onAddKey: () => void
  onDeleteKey: (k: ProviderKeyView) => void
}> = ({ models, keys, onAddModel, onDeleteModel, onToggleModel, onAddKey, onDeleteKey }) => (
  <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <section>
      <div className="flex items-center justify-between mb-2">
        <h3 className="font-semibold text-gray-700 text-sm">Models</h3>
        <button
          onClick={onAddModel}
          className="text-xs px-2 py-1 bg-indigo-600 text-white rounded hover:bg-indigo-700"
        >
          + Add model
        </button>
      </div>
      {models.length === 0 ? (
        <div className="text-xs text-gray-500">No models registered.</div>
      ) : (
        <table className="w-full text-xs">
          <thead className="text-gray-500">
            <tr>
              <th className="text-left py-1">Model</th>
              <th className="text-left py-1">Capabilities</th>
              <th className="text-left py-1 w-20">Enabled</th>
              <th className="py-1 w-12" />
            </tr>
          </thead>
          <tbody>
            {models.map((m) => (
              <tr key={m.id}>
                <td className="py-1 font-mono">{m.model_name}</td>
                <td className="py-1 text-gray-600">
                  {m.supports_streaming ? 'stream ' : ''}
                  {m.supports_tools ? 'tools ' : ''}
                  {m.supports_vision ? 'vision ' : ''}
                </td>
                <td className="py-1">
                  <Toggle on={!!m.enabled} onChange={() => onToggleModel(m)} />
                </td>
                <td className="py-1 text-right">
                  <button
                    onClick={() => onDeleteModel(m)}
                    className="text-red-600 hover:text-red-800"
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
    <section>
      <div className="flex items-center justify-between mb-2">
        <h3 className="font-semibold text-gray-700 text-sm">API keys</h3>
        <button
          onClick={onAddKey}
          className="text-xs px-2 py-1 bg-indigo-600 text-white rounded hover:bg-indigo-700"
        >
          + Add key
        </button>
      </div>
      {keys.length === 0 ? (
        <div className="text-xs text-gray-500">No keys registered.</div>
      ) : (
        <table className="w-full text-xs">
          <thead className="text-gray-500">
            <tr>
              <th className="text-left py-1">Preview</th>
              <th className="text-left py-1">Label</th>
              <th className="text-left py-1 w-16">Priority</th>
              <th className="py-1 w-12" />
            </tr>
          </thead>
          <tbody>
            {keys.map((k) => (
              <tr key={k.id}>
                <td className="py-1 font-mono">{k.key_preview}</td>
                <td className="py-1">{k.label ?? '—'}</td>
                <td className="py-1">{k.priority}</td>
                <td className="py-1 text-right">
                  <button
                    onClick={() => onDeleteKey(k)}
                    className="text-red-600 hover:text-red-800"
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  </div>
)

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
