import React from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'

const NAV = [
  { to: '/dashboard', label: 'Overview' },
  { to: '/providers', label: 'Providers' },
  { to: '/aliases', label: 'Aliases' },
  { to: '/api-keys', label: 'API Keys' },
  { to: '/keys', label: 'Key Pools' },
  { to: '/vision', label: 'Vision' },
]

export const AppLayout: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const navigate = useNavigate()
  const location = useLocation()
  const username = localStorage.getItem('username') || 'admin'

  const handleLogout = () => {
    localStorage.removeItem('jwt_token')
    localStorage.removeItem('tenant_id')
    localStorage.removeItem('username')
    navigate('/')
  }

  return (
    <div className="min-h-screen bg-gray-50 flex flex-col">
      <header className="bg-white border-b border-gray-200 shadow-sm">
        <div className="px-6 py-3 flex items-center justify-between">
          <div className="flex items-center gap-6">
            <Link to="/dashboard" className="text-xl font-bold text-indigo-600">
              LLMProxy
            </Link>
            <nav className="flex gap-1">
              {NAV.map((item) => {
                const active = location.pathname.startsWith(item.to)
                return (
                  <Link
                    key={item.to}
                    to={item.to}
                    className={`px-3 py-1.5 rounded-md text-sm font-medium transition ${
                      active
                        ? 'bg-indigo-50 text-indigo-700'
                        : 'text-gray-600 hover:bg-gray-100 hover:text-gray-900'
                    }`}
                  >
                    {item.label}
                  </Link>
                )
              })}
            </nav>
          </div>
          <div className="flex items-center gap-4">
            <span className="text-sm text-gray-500">{username}</span>
            <button
              onClick={handleLogout}
              className="px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-100 rounded-md transition"
            >
              Logout
            </button>
          </div>
        </div>
      </header>
      <main className="flex-1 max-w-7xl w-full mx-auto px-6 py-8">{children}</main>
    </div>
  )
}
