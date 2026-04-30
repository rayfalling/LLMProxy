import { defineConfig, devices } from '@playwright/test'
import * as path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const PORT = Number(process.env.E2E_PORT) || 18081
const BASE_URL = `http://127.0.0.1:${PORT}`

// Locate the dashboard binary built by `cargo build -p dashboard`.
// Allow override via DASHBOARD_BIN for CI / cross-platform layouts.
const dashboardBin =
  process.env.DASHBOARD_BIN ||
  path.resolve(
    __dirname,
    '..',
    'target',
    process.platform === 'win32' ? 'debug/dashboard.exe' : 'debug/dashboard',
  )

const tempDb = path.resolve(__dirname, 'test-data', 'e2e.db')

export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  timeout: 30_000,

  use: {
    baseURL: BASE_URL,
    trace: 'retain-on-failure',
    ...devices['Desktop Chrome'],
  },

  webServer: {
    // The helper scripts (scripts/e2e.{ps1,sh}) wipe `web/test-data/`
    // before each run, so the dashboard always starts against an empty
    // database — the precondition the setup-wizard flow depends on.
    command: `"${dashboardBin}"`,
    url: `${BASE_URL}/healthz`,
    timeout: 30_000,
    reuseExistingServer: false,
    env: {
      DATABASE_URL: `sqlite://${tempDb.replace(/\\/g, '/')}?mode=rwc`,
      DASHBOARD_HOST: '127.0.0.1',
      DASHBOARD_PORT: String(PORT),
      JWT_SECRET: 'e2e-test-secret-please-do-not-use-in-prod',
      RUST_LOG: 'dashboard=warn',
    },
  },
})
