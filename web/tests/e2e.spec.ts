import { test, expect, Page } from '@playwright/test'

// All five E2E specs run sequentially in a single serial flow because
// they share state (a single dashboard process + SQLite file) — the
// helper script resets the DB before each invocation.
test.describe.configure({ mode: 'serial' })

const TENANT = 'e2e-tenant'
const USERNAME = 'e2e-admin'
const PASSWORD = 'correct horse battery staple'

async function loginAs(page: Page) {
  await page.goto('/')
  await page.getByLabel(/tenant/i).fill(TENANT)
  await page.getByLabel(/username/i).fill(USERNAME)
  await page.getByLabel(/password/i).fill(PASSWORD)
  await page.getByRole('button', { name: /log\s*in|sign\s*in|login/i }).click()
  await page.waitForURL((url) => url.pathname === '/dashboard', { timeout: 10_000 })
}

test.describe('first-boot setup → login → CRUD pages', () => {
  test('6.1 visits root with empty DB → SetupGuard redirects to /setup', async ({ page }) => {
    await page.goto('/')
    await page.waitForURL((url) => url.pathname === '/setup', { timeout: 10_000 })
    await expect(page.getByRole('heading', { name: /setup|first.?boot|welcome/i })).toBeVisible()
  })

  test('6.1 setup wizard validates and creates the first tenant + admin', async ({ page }) => {
    await page.goto('/setup')

    // submit empty form → client-side validation surfaces an error
    await page.getByRole('button', { name: /create|setup|submit/i }).click()
    await expect(page.getByText(/required|missing/i).first()).toBeVisible()

    // fill happy path
    await page.getByLabel(/tenant/i).fill(TENANT)
    await page.getByLabel(/username/i).fill(USERNAME)
    await page.getByLabel(/^password$/i).fill(PASSWORD)
    await page.getByLabel(/confirm/i).fill(PASSWORD)
    await page.getByRole('button', { name: /create|setup|submit/i }).click()

    // The Setup page shows a success screen for 2s before navigate('/').
    // Wait for that redirect to actually fire so subsequent tests see a
    // fully-initialised database.
    await page.waitForURL((url) => url.pathname === '/', { timeout: 15_000 })
  })

  test('6.5 /setup is locked once initialized', async ({ page }) => {
    await page.goto('/setup')
    // SetupGuard should bounce us back to /
    await page.waitForURL((url) => url.pathname === '/', { timeout: 10_000 })
    await expect(page).toHaveURL(/\/$/)
  })

  test('6.2 login with the freshly created admin lands on /dashboard', async ({ page }) => {
    await loginAs(page)
    await expect(page.getByRole('heading', { name: /overview/i })).toBeVisible()
  })

  test('6.3 /providers loads (empty-state expected on a fresh DB)', async ({ page }) => {
    await loginAs(page)
    await page.goto('/providers')
    await expect(page.getByRole('heading', { name: /providers/i })).toBeVisible()
    // empty-DB invariant: the providers <tbody> exists but has 0 data rows
    const rows = page.locator('tbody tr')
    await expect(rows).toHaveCount(0)
  })

  test('6.4 /aliases loads (empty-state expected on a fresh DB)', async ({ page }) => {
    await loginAs(page)
    await page.goto('/aliases')
    await expect(page.getByRole('heading', { name: /alias/i })).toBeVisible()
    await expect(page.getByText(/no aliases configured/i)).toBeVisible()
  })
})
