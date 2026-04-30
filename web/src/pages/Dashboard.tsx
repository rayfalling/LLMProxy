import React, { useEffect, useState } from 'react'
import { AppLayout } from '../components/AppLayout'
import { apiClient } from '../api/client'
import { TenantStats, FailoverEventView } from '../api/types'

export const Dashboard: React.FC = () => {
  const [stats, setStats] = useState<TenantStats | null>(null)
  const [events, setEvents] = useState<FailoverEventView[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    Promise.all([apiClient.getTenantStats(), apiClient.listFailoverEvents(10)])
      .then(([s, e]) => {
        if (cancelled) return
        setStats(s)
        setEvents(e)
      })
      .catch((err) => {
        if (cancelled) return
        setError(err.response?.data?.message || 'Failed to load dashboard')
      })
      .finally(() => !cancelled && setLoading(false))
    return () => {
      cancelled = true
    }
  }, [])

  return (
    <AppLayout>
      <h1 className="text-2xl font-bold text-gray-900 mb-6">Overview</h1>

      {loading && <div className="text-gray-500">Loading…</div>}
      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
          {error}
        </div>
      )}

      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
          <StatCard label="QPS (1h)" value={stats.qps_last_hour.toFixed(2)} unit="req/s" />
          <StatCard label="P50 latency (1h)" value={stats.p50_latency_ms_last_hour.toFixed(0)} unit="ms" />
          <StatCard label="P95 latency (1h)" value={stats.p95_latency_ms_last_hour.toFixed(0)} unit="ms" />
          <StatCard label="Error rate (1h)" value={(stats.error_rate_last_hour * 100).toFixed(2)} unit="%" />
          <StatCard label="Total requests" value={stats.total_requests.toLocaleString()} unit="" />
          <StatCard label="Failover events (1h)" value={stats.failover_count_last_hour.toString()} unit="" />
          <StatCard label="Avg latency" value={stats.avg_latency_ms.toFixed(0)} unit="ms" />
          <StatCard
            label="Total tokens"
            value={(stats.total_input_tokens + stats.total_output_tokens).toLocaleString()}
            unit=""
          />
        </div>
      )}

      <div className="bg-white rounded-lg shadow">
        <div className="px-6 py-4 border-b border-gray-200">
          <h2 className="text-lg font-semibold text-gray-900">Recent failover events</h2>
        </div>
        {events.length === 0 ? (
          <div className="px-6 py-8 text-center text-gray-500">No failover events recorded.</div>
        ) : (
          <table className="w-full text-sm">
            <thead className="bg-gray-50 text-gray-600">
              <tr>
                <th className="px-6 py-2 text-left">Time</th>
                <th className="px-6 py-2 text-left">Alias</th>
                <th className="px-6 py-2 text-left">Provider / Model</th>
                <th className="px-6 py-2 text-right">Failovers</th>
                <th className="px-6 py-2 text-left">Error</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {events.map((e) => (
                <tr key={e.request_id}>
                  <td className="px-6 py-2 text-gray-500">{e.created_at}</td>
                  <td className="px-6 py-2 font-medium">{e.model_alias}</td>
                  <td className="px-6 py-2">
                    {e.provider_id || '-'}
                    {e.provider_model ? ` / ${e.provider_model}` : ''}
                  </td>
                  <td className="px-6 py-2 text-right">{e.failover_count}</td>
                  <td className="px-6 py-2 text-red-600">{e.error_code || '-'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </AppLayout>
  )
}

const StatCard: React.FC<{ label: string; value: string; unit: string }> = ({ label, value, unit }) => (
  <div className="bg-white rounded-lg p-4 shadow-sm border border-gray-100">
    <p className="text-xs uppercase tracking-wide text-gray-500">{label}</p>
    <p className="mt-1 text-2xl font-bold text-gray-900">
      {value}
      {unit && <span className="ml-1 text-sm text-gray-500">{unit}</span>}
    </p>
  </div>
)
