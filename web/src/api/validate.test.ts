import { describe, it, expect } from 'vitest'
import { validateSetup, SETUP_ERROR_MESSAGES } from './validate'

const valid = {
  tenant_name: 'acme',
  username: 'admin',
  password: 'longenough',
  password_confirm: 'longenough',
}

describe('validateSetup', () => {
  it('accepts a valid request', () => {
    expect(validateSetup(valid)).toBeNull()
  })

  it('rejects empty tenant_name (incl. whitespace-only)', () => {
    expect(validateSetup({ ...valid, tenant_name: '' })).toBe('tenant_name_required')
    expect(validateSetup({ ...valid, tenant_name: '   ' })).toBe('tenant_name_required')
  })

  it('rejects empty username', () => {
    expect(validateSetup({ ...valid, username: '' })).toBe('username_required')
    expect(validateSetup({ ...valid, username: '\t' })).toBe('username_required')
  })

  it('rejects passwords shorter than 8 chars', () => {
    expect(
      validateSetup({ ...valid, password: '7chars7', password_confirm: '7chars7' }),
    ).toBe('password_too_short')
  })

  it('rejects mismatched passwords', () => {
    expect(
      validateSetup({ ...valid, password: 'alpha1234', password_confirm: 'beta12345' }),
    ).toBe('password_mismatch')
  })

  it('exposes a message for every error code', () => {
    for (const code of [
      'tenant_name_required',
      'username_required',
      'password_too_short',
      'password_mismatch',
    ] as const) {
      expect(SETUP_ERROR_MESSAGES[code]).toBeTruthy()
    }
  })
})
