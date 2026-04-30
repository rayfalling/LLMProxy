import { SetupRequest } from './types'

export type SetupValidationError =
  | 'tenant_name_required'
  | 'username_required'
  | 'password_too_short'
  | 'password_mismatch'

export const SETUP_ERROR_MESSAGES: Record<SetupValidationError, string> = {
  tenant_name_required: 'Tenant name is required',
  username_required: 'Username is required',
  password_too_short: 'Password must be at least 8 characters',
  password_mismatch: 'Passwords do not match',
}

/**
 * Pure client-side validation for the first-boot setup form.
 * Returns null when the request is valid, or a stable error code that
 * UI code can map to a localized message via SETUP_ERROR_MESSAGES.
 */
export function validateSetup(req: SetupRequest): SetupValidationError | null {
  if (!req.tenant_name.trim()) return 'tenant_name_required'
  if (!req.username.trim()) return 'username_required'
  if (req.password.length < 8) return 'password_too_short'
  if (req.password !== req.password_confirm) return 'password_mismatch'
  return null
}
