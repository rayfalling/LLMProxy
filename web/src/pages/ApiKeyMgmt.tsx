import React, { useEffect, useState } from 'react'
import { AppLayout } from '../components/AppLayout'
import { ResourceCreateModal, ModalField } from '../components/ResourceCreateModal'
import { ConfirmDeleteDialog } from '../components/ConfirmDeleteDialog'
import { RevealOnceModal } from '../components/RevealOnceModal'
import { apiClient } from '../api/client'
import { ApiKeyView } from '../api/types'

export const ApiKeyMgmt: React.FC = () => {
  const [keys, setKeys] = useState<ApiKeyView[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [showCreate, setShowCreate] = useState(false)
  const [pendingDelete, setPendingDelete] = useState<ApiKeyView | null>(null)
  const [revealKey, setRevealKey] = useState<string | null>(null)

  const load = async () => {
    try {
      setKeys(await apiClient.listApiKeys())
    } catch (e: any) {
      setError(e.response?.data?.message || 'Failed to load API keys')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    load()
  }, [])

  const fields: ModalField[] = [
    { name: 'name', label: 'Name (optional)', placeholder: 'e.g. ci-pipeline' },
    { name: 'quota_rpm', label: 'Quota: requests/min (optional)', type: 'number' },
    { name: 'quota_tpm', label: 'Quota: tokens/min (optional)', type: 'number' },
    { name: 'quota_daily_req', label: 'Quota: requests/day (optional)', type: 'number' },
  ]

  return (
    <AppLayout>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900">API keys</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="px-3 py-1.5 text-sm bg-indigo-600 text-white rounded-md hover:bg-indigo-700 transition"
        >
          + Issue new key
        </button>
      </div>
      <p className="text-sm text-gray-500 mb-4">
        Each API key authenticates a client to the proxy. The full key is shown <strong>once</strong>{' '}
        at creation time — store it somewhere safe.
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
      ) : keys.length === 0 ? (
        <div className="bg-white border-2 border-dashed border-gray-200 rounded-lg p-12 text-center text-gray-500">
          No API keys yet. Click <strong>Issue new key</strong> to create one.
        </div>
      ) : (
        <div className="bg-white rounded-lg shadow overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-gray-50 text-gray-600">
              <tr>
                <th className="px-6 py-3 text-left">Name</th>
                <th className="px-6 py-3 text-left">Prefix</th>
                <th className="px-6 py-3 text-left">Status</th>
                <th className="px-6 py-3 text-left">Quotas (rpm / tpm / daily)</th>
                <th className="px-6 py-3 text-left">Created</th>
                <th className="px-6 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {keys.map((k) => (
                <tr key={k.id}>
                  <td className="px-6 py-3">{k.name ?? '—'}</td>
                  <td className="px-6 py-3 font-mono text-xs">{k.prefix}…</td>
                  <td className="px-6 py-3">
                    <StatusBadge status={k.status} />
                  </td>
                  <td className="px-6 py-3 font-mono text-xs">
                    {k.quota_rpm ?? '∞'} / {k.quota_tpm ?? '∞'} / {k.quota_daily_req ?? '∞'}
                  </td>
                  <td className="px-6 py-3 text-xs text-gray-500">{k.created_at}</td>
                  <td className="px-6 py-3 text-right">
                    {k.status === 'active' && (
                      <button
                        onClick={() => setPendingDelete(k)}
                        className="text-red-600 hover:text-red-800 text-sm font-medium"
                      >
                        Revoke
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {showCreate && (
        <ResourceCreateModal
          title="Issue new API key"
          fields={fields}
          submitLabel="Create"
          onCancel={() => setShowCreate(false)}
          onSubmit={async (v) => {
            const created = await apiClient.createApiKey({
              name: v.name || null,
              quota_rpm: v.quota_rpm ? Number(v.quota_rpm) : null,
              quota_tpm: v.quota_tpm ? Number(v.quota_tpm) : null,
              quota_daily_req: v.quota_daily_req ? Number(v.quota_daily_req) : null,
            })
            setShowCreate(false)
            setRevealKey(created.plaintext_key)
            await load()
          }}
        />
      )}
      {revealKey && (
        <RevealOnceModal
          title="API key created"
          secret={revealKey}
          helpText="Pass this token as a Bearer header to /v1/* endpoints on the proxy."
          onClose={() => setRevealKey(null)}
        />
      )}
      {pendingDelete && (
        <ConfirmDeleteDialog
          title="Revoke API key"
          message={
            <span>
              Revoke key <code>{pendingDelete.prefix}…</code>? Existing clients using this key
              will start receiving 401 immediately.
            </span>
          }
          confirmText="Revoke"
          onCancel={() => setPendingDelete(null)}
          onConfirm={async () => {
            await apiClient.deleteApiKey(pendingDelete.id)
            setPendingDelete(null)
            await load()
          }}
        />
      )}
    </AppLayout>
  )
}

const StatusBadge: React.FC<{ status: string }> = ({ status }) => {
  const cls =
    status === 'active'
      ? 'bg-green-100 text-green-800'
      : status === 'revoked'
      ? 'bg-red-100 text-red-800'
      : 'bg-gray-100 text-gray-700'
  return <span className={`px-2 py-0.5 rounded text-xs font-medium ${cls}`}>{status}</span>
}
