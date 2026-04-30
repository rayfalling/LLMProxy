import React, { useEffect, useState } from 'react'
import { Header } from '../components/Header'
import { apiClient } from '../api/client'
import { TenantStats, FailoverEvent } from '../api/types'

export const Dashboard: React.FC = () => {
  const [stats, setStats] = useState<TenantStats | null>(null)
  const [events, setEvents] = useState<FailoverEvent[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [statsData, eventsData] = await Promise.all([
          apiClient.getTenantStats(),
          apiClient.listFailoverEvents(10),
        ])
        setStats(statsData)
        setEvents(eventsData)
      } catch (err) {
        console.error('Failed to fetch dashboard data:', err)
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [])

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Header />
        <div className="flex items-center justify-center h-96">
          <div className="text-lg text-gray-600">Loading...</div>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Header />
      <main className="max-w-7xl mx-auto px-6 py-8">
        {/* Metrics Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          <StatCard
            title="QPS"
            value={stats?.qps.toFixed(1) || '0.0'}
            unit="req/s"
            className="bg-blue-50"
          />
          <StatCard
            title="P50 Latency"
            value={stats?.p50_latency_ms.toFixed(0) || '0'}
            unit="ms"
            className="bg-green-50"
          />
          <StatCard
            title="P95 Latency"
            value={stats?.p95_latency_ms.toFixed(0) || '0'}
            unit="ms"
            className="bg-yellow-50"
          />
          <StatCard
            title="Error Rate"
            value={stats?.error_rate.toFixed(2) || '0.00'}
            unit="%"
            className="bg-red-50"
          />
        </div>

        {/* Failover Events */}
        <div className="bg-white rounded-lg shadow">
          <div className="px-6 py-4 border-b border-gray-200">
            <h2 className="text-lg font-semibold text-gray-900">Recent Failover Events</h2>
          </div>
          <div className="divide-y divide-gray-200">
            {events.length === 0 ? (
              <div className="px-6 py-8 text-center text-gray-500">No failover events</div>
            ) : (
              events.map((event) => (
                <div key={event.id} className="px-6 py-4">
                  <div className="flex items-start justify-between">
                    <div>
                      <p className="font-medium text-gray-900">{event.alias_name}</p>
                      <p className="text-sm text-gray-600">
                        {event.original_provider} → {event.failover_provider}
                      </p>
                      <p className="text-xs text-gray-500 mt-1">{event.reason}</p>
                    </div>
                    <span className="text-xs text-gray-500">
                      {new Date(event.created_at).toLocaleString()}
                    </span>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Navigation Links */}
        <div className="mt-8 grid grid-cols-2 md:grid-cols-4 gap-4">
          <NavLink href="/providers" icon="🔌" title="Providers" />
          <NavLink href="/aliases" icon="🔀" title="Aliases" />
          <NavLink href="/keys" icon="🔑" title="Key Pools" />
          <NavLink href="/vision" icon="👁️" title="Vision Mapping" />
        </div>
      </main>
    </div>
  )
}

interface StatCardProps {
  title: string
  value: string
  unit: string
  className: string
}

const StatCard: React.FC<StatCardProps> = ({ title, value, unit, className }) => (
  <div className={`${className} rounded-lg p-6`}>
    <p className="text-sm font-medium text-gray-600">{title}</p>
    <p className="text-3xl font-bold text-gray-900 mt-2">
      {value} <span className="text-base text-gray-600">{unit}</span>
    </p>
  </div>
)

interface NavLinkProps {
  href: string
  icon: string
  title: string
}

const NavLink: React.FC<NavLinkProps> = ({ href, icon, title }) => (
  <a
    href={href}
    className="bg-white rounded-lg p-6 shadow hover:shadow-lg transition text-center"
  >
    <div className="text-3xl mb-3">{icon}</div>
    <p className="font-semibold text-gray-900">{title}</p>
  </a>
)
