import { useEffect, useState } from 'react'
import {
  BrowserRouter as Router,
  Routes,
  Route,
  Navigate,
  useLocation,
  useNavigate,
} from 'react-router-dom'
import { Login } from './pages/Login'
import { Setup } from './pages/Setup'
import { Dashboard } from './pages/Dashboard'
import { ProviderMgmt } from './pages/ProviderMgmt'
import { AliasMgmt } from './pages/AliasMgmt'
import { KeyPoolMgmt } from './pages/KeyPoolMgmt'
import { VisionMgmt } from './pages/VisionMgmt'
import { ProtectedRoute } from './components/ProtectedRoute'
import { apiClient } from './api/client'

function SetupGuard({ children }: { children: React.ReactNode }) {
  const location = useLocation()
  const navigate = useNavigate()
  const [checking, setChecking] = useState(true)

  useEffect(() => {
    let cancelled = false
    apiClient
      .getSetupStatus()
      .then(({ initialized }) => {
        if (cancelled) return
        if (!initialized && location.pathname !== '/setup') {
          navigate('/setup', { replace: true })
        } else if (initialized && location.pathname === '/setup') {
          navigate('/', { replace: true })
        }
      })
      .catch(() => {
        // If we can't reach the API, fall through and let the page render
        // (login or setup form will surface the error to the user).
      })
      .finally(() => {
        if (!cancelled) setChecking(false)
      })
    return () => {
      cancelled = true
    }
  }, [location.pathname, navigate])

  if (checking) {
    return (
      <div className="min-h-screen flex items-center justify-center text-gray-500">
        Loading…
      </div>
    )
  }
  return <>{children}</>
}

function App() {
  return (
    <Router>
      <SetupGuard>
        <Routes>
          <Route path="/" element={<Login />} />
          <Route path="/setup" element={<Setup />} />
          <Route
            path="/dashboard"
            element={
              <ProtectedRoute>
                <Dashboard />
              </ProtectedRoute>
            }
          />
          <Route
            path="/providers"
            element={
              <ProtectedRoute>
                <ProviderMgmt />
              </ProtectedRoute>
            }
          />
          <Route
            path="/aliases"
            element={
              <ProtectedRoute>
                <AliasMgmt />
              </ProtectedRoute>
            }
          />
          <Route
            path="/keys"
            element={
              <ProtectedRoute>
                <KeyPoolMgmt />
              </ProtectedRoute>
            }
          />
          <Route
            path="/vision"
            element={
              <ProtectedRoute>
                <VisionMgmt />
              </ProtectedRoute>
            }
          />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </SetupGuard>
    </Router>
  )
}

export default App
