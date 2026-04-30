import React, { useEffect, useState } from 'react'
import { AppLayout } from '../components/AppLayout'
import { apiClient } from '../api/client'
import { AliasView } from '../api/types'

const STRATEGIES = ['priority', 'latency', 'cost']

export const AliasMgmt: React.FC = () => {
  const [aliases, setAliases] = useState<AliasView[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [savingId, setSavingId] = useState<string | null>(null)

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
      <h1 className="text-2xl font-bold text-gray-900 mb-6">Model aliases</h1>
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
    </AppLayout>
  )
}
