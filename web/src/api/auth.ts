import { InternalAxiosRequestConfig, AxiosError } from 'axios'

export const TOKEN_STORAGE_KEY = 'jwt_token'

/**
 * Attach `Authorization: Bearer <token>` to outgoing axios requests when
 * a JWT is present in localStorage. Pure function — exported so tests
 * can call it without spinning up a network stack.
 */
export function attachAuthHeader(
  config: InternalAxiosRequestConfig,
): InternalAxiosRequestConfig {
  const token = localStorage.getItem(TOKEN_STORAGE_KEY)
  if (token) {
    config.headers.set('Authorization', `Bearer ${token}`)
  }
  return config
}

/**
 * Decide where the SPA should bounce to on a 401 response.
 * Stays on /setup and / (the public surfaces) so the user is not
 * redirected away from a form they're filling in. Returns the new
 * pathname or null if the SPA should remain on the current path.
 */
export function resolveUnauthorizedRedirect(
  currentPath: string,
): string | null {
  if (currentPath === '/' || currentPath === '/setup') return null
  return '/'
}

/**
 * Default 401 handler used by the axios response interceptor.
 * Drops the cached JWT and (when appropriate) navigates back to the
 * login page. Exposed so unit tests can drive it directly.
 */
export function handleUnauthorized(
  error: AxiosError,
  storage: Pick<Storage, 'removeItem'> = localStorage,
  loc: { pathname: string; href: string } = window.location,
): Promise<never> {
  if (error.response?.status === 401) {
    storage.removeItem(TOKEN_STORAGE_KEY)
    const target = resolveUnauthorizedRedirect(loc.pathname)
    if (target) loc.href = target
  }
  return Promise.reject(error)
}
