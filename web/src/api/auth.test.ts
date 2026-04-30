import { describe, it, expect, beforeEach } from 'vitest'
import type { AxiosError, InternalAxiosRequestConfig } from 'axios'
import { AxiosHeaders } from 'axios'
import {
  attachAuthHeader,
  handleUnauthorized,
  resolveUnauthorizedRedirect,
  TOKEN_STORAGE_KEY,
} from './auth'

function buildConfig(): InternalAxiosRequestConfig {
  return { headers: new AxiosHeaders() } as InternalAxiosRequestConfig
}

describe('attachAuthHeader', () => {
  beforeEach(() => localStorage.clear())

  it('does nothing when no token is stored', () => {
    const cfg = attachAuthHeader(buildConfig())
    expect(cfg.headers.get('Authorization')).toBeUndefined()
  })

  it('injects Bearer token when present in localStorage', () => {
    localStorage.setItem(TOKEN_STORAGE_KEY, 'abc.def.ghi')
    const cfg = attachAuthHeader(buildConfig())
    expect(cfg.headers.get('Authorization')).toBe('Bearer abc.def.ghi')
  })
})

describe('resolveUnauthorizedRedirect', () => {
  it('keeps the user on public pages', () => {
    expect(resolveUnauthorizedRedirect('/')).toBeNull()
    expect(resolveUnauthorizedRedirect('/setup')).toBeNull()
  })

  it('redirects authenticated pages back to login', () => {
    expect(resolveUnauthorizedRedirect('/dashboard')).toBe('/')
    expect(resolveUnauthorizedRedirect('/providers')).toBe('/')
  })
})

describe('handleUnauthorized', () => {
  beforeEach(() => localStorage.clear())

  function makeError(status: number): AxiosError {
    return { response: { status } } as AxiosError
  }

  it('clears token and redirects on 401 from a protected page', async () => {
    localStorage.setItem(TOKEN_STORAGE_KEY, 'tok')
    const loc = { pathname: '/dashboard', href: '/dashboard' }
    await expect(handleUnauthorized(makeError(401), localStorage, loc)).rejects.toBeDefined()
    expect(localStorage.getItem(TOKEN_STORAGE_KEY)).toBeNull()
    expect(loc.href).toBe('/')
  })

  it('does not redirect when 401 fires on the login page', async () => {
    localStorage.setItem(TOKEN_STORAGE_KEY, 'tok')
    const loc = { pathname: '/', href: '/' }
    await expect(handleUnauthorized(makeError(401), localStorage, loc)).rejects.toBeDefined()
    expect(localStorage.getItem(TOKEN_STORAGE_KEY)).toBeNull()
    expect(loc.href).toBe('/')
  })

  it('passes non-401 errors through unchanged', async () => {
    localStorage.setItem(TOKEN_STORAGE_KEY, 'tok')
    const loc = { pathname: '/dashboard', href: '/dashboard' }
    await expect(handleUnauthorized(makeError(500), localStorage, loc)).rejects.toBeDefined()
    expect(localStorage.getItem(TOKEN_STORAGE_KEY)).toBe('tok')
    expect(loc.href).toBe('/dashboard')
  })
})
