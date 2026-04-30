import React, { useEffect, useState } from 'react'
import { AppLayout } from '../components/AppLayout'
import { apiClient } from '../api/client'
import { KeyPoolMappingView } from '../api/types'

export const KeyPoolMgmt: React.FC = () => {
  const [rows, setRows] = useState<KeyPoolMappingView[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    apiClient
      .listKeyPoolMappings()
      .then(setRows)
      .catch((e) => setError(e.response?.data?.message || 'Failed to load key pools'))
      .finally(() => setLoading(false))
  }, [])

  // Group flat (api_key_id, provider_key_id) pairs into a per-api-key view.
  const grouped = React.useMemo(() => {
    const map = new Map<string, string[]>()
    rows.forEach((r) => {
      const arr = map.get(r.api_key_id) || []
      arr.push(r.provider_key_id)
      map.set(r.api_key_id, arr)
    })
    return Array.from(map.entries())
  }, [rows])

  return (
    <AppLayout>
      <h1 className="text-2xl font-bold text-gray-900 mb-6">Key pools</h1>
      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4">
          {error}
        </div>
      )}
      {loading ? (
        <div className="text-gray-500">Loading…</div>
      ) : grouped.length === 0 ? (
        <div className="bg-white rounded-lg shadow px-6 py-8 text-center text-gray-500">
          No key-pool mappings configured for this tenant.
        </div>
      ) : (
        <div className="bg-white rounded-lg shadow overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-gray-50 text-gray-600">
              <tr>
                <th className="px-6 py-3 text-left">Inbound API key</th>
                <th className="px-6 py-3 text-left">Provider keys</th>
                <th className="px-6 py-3 text-right">#</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {grouped.map(([apiKeyId, providerKeyIds]) => (
                <tr key={apiKeyId}>
                  <td className="px-6 py-3 font-mono text-xs">{apiKeyId}</td>
                  <td className="px-6 py-3">
                    <div className="flex flex-wrap gap-1">
                      {providerKeyIds.map((id) => (
                        <span
                          key={id}
                          className="bg-indigo-50 text-indigo-700 text-xs font-mono px-2 py-0.5 rounded"
                        >
                          {id}
                        </span>
                      ))}
                    </div>
                  </td>
                  <td className="px-6 py-3 text-right text-gray-600">{providerKeyIds.length}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="mt-4 text-sm text-gray-500">
        Editing the binding set is available via{' '}
        <code className="font-mono text-xs">PUT /api/key-pools/&lt;api_key_id&gt;</code>; an
        in-place editor will land alongside provider-key CRUD in a follow-up.
      </p>
    </AppLayout>
  )
}
