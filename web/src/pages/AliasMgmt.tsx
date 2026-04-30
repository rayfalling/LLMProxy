import React, { useEffect, useState } from 'react'
import { AppLayout } from '../components/AppLayout'
import { ResourceCreateModal, ModalField } from '../components/ResourceCreateModal'
import { ConfirmDeleteDialog } from '../components/ConfirmDeleteDialog'
import { apiClient } from '../api/client'
import { AliasView } from '../api/types'

const STRATEGIES = ['priority', 'latency', 'cost']

export const AliasMgmt: React.FC = () => {
  const [aliases, setAliases] = useState<AliasView[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [savingId, setSavingId] = useState<string | null>(null)
  const [showCreate, setShowCreate] = useState(false)
  const [pendingDelete, setPendingDelete] = useState<AliasView | null>(null)

  const fields: ModalField[] = [
    {
      name: 'alias_name',
      label: 'Alias name',
      required: true,
      placeholder: 'e.g. gpt-4o',
      helpText: 'Clients refer to the model by this alias.',
    },
    { name: 'description', label: 'Description (optional)' },
    {
      name: 'route_strategy',
      label: 'Route strategy',
      type: 'select',
      defaultValue: 'priority',
      options: STRATEGIES.map((s) => ({ value: s, label: s })),
    },
  ]

  const load = async () => {
    try {
      setAliases(await apiClient.listAliases())
    } catch (e: any) {
      setError(e.response?.data?.message || 'Failed to load aliases')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    load()
  }, [])

  const onChangeStrategy = async (a: AliasView, strategy: string) => {
    setSavingId(a.id)
    try {
      await apiClient.updateAliasStrategy(a.alias_name, strategy)
      await load()
    } catch (e: any) {
      setError(e.response?.data?.message || 'Failed to update strategy')
    } finally {
      setSavingId(null)
    }
  }

  return (
    <AppLayout>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900">Model aliases</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="px-3 py-1.5 text-sm bg-indigo-600 text-white rounded-md hover:bg-indigo-700 transition"
        >
          + Add alias
        </button>
      </div>
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
                <th className="px-6 py-3 text-left">Alias</th>
                <th className="px-6 py-3 text-left">Description</th>
                <th className="px-6 py-3 text-left">Route strategy</th>
                <th className="px-6 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {aliases.map((a) => (
                <tr key={a.id}>
                  <td className="px-6 py-3 font-mono text-xs">{a.alias_name}</td>
                  <td className="px-6 py-3 text-gray-600">{a.description || '-'}</td>
                  <td className="px-6 py-3">
                    <select
                      value={a.route_strategy}
                      disabled={savingId === a.id}
                      onChange={(e) => onChangeStrategy(a, e.target.value)}
                      className="border border-gray-300 rounded px-2 py-1 text-sm focus:ring-indigo-500 focus:border-indigo-500"
                    >
                      {STRATEGIES.map((s) => (
                        <option key={s} value={s}>
                          {s}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="px-6 py-3 text-right">
                    <button
                      onClick={() => setPendingDelete(a)}
                      className="text-red-600 hover:text-red-800 text-sm font-medium"
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {aliases.length === 0 && (
            <div className="px-6 py-8 text-center text-gray-500">No aliases configured.</div>
          )}
        </div>
      )}
      <p className="mt-4 text-sm text-gray-500">
        Targets management (priority + provider/model bindings) is exposed via{' '}
        <code className="font-mono text-xs">PUT /api/aliases/&lt;alias&gt;/targets</code> and will
        gain a UI in a follow-up iteration.
      </p>
      {showCreate && (
        <ResourceCreateModal
          title="Add alias"
          fields={fields}
          onCancel={() => setShowCreate(false)}
          onSubmit={async (v) => {
            await apiClient.createAlias({
              alias_name: v.alias_name,
              description: v.description || null,
              route_strategy: v.route_strategy || 'priority',
              targets: [],
            })
            setShowCreate(false)
            await load()
          }}
        />
      )}
      {pendingDelete && (
        <ConfirmDeleteDialog
          title="Delete alias"
          message={
            <span>
              Permanently delete alias <code>{pendingDelete.alias_name}</code> and all of its
              targets and failover rules?
            </span>
          }
          confirmText="Delete"
          typedConfirmation={pendingDelete.alias_name}
          onCancel={() => setPendingDelete(null)}
          onConfirm={async () => {
            await apiClient.deleteAlias(pendingDelete.alias_name)
            setPendingDelete(null)
            await load()
          }}
        />
      )}
    </AppLayout>
  )
}
