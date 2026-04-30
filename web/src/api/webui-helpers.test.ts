import { describe, it, expect } from 'vitest'
import { initialValuesFromFields, groupKeyPoolMappings } from './webui-helpers'
import type { ModalField } from '../components/ResourceCreateModal'
import type { KeyPoolMappingView, ProviderKeyView } from './types'

const pk = (id: string, providerId: string): ProviderKeyView => ({
  id,
  provider_id: providerId,
  label: null,
  enabled: 1,
  priority: 0,
  key_preview: `sk-${id}`,
})

describe('initialValuesFromFields', () => {
  it('returns empty strings when no defaults are provided', () => {
    const fields: ModalField[] = [
      { name: 'name', label: 'Name' },
      { name: 'url', label: 'URL', type: 'url' },
    ]
    expect(initialValuesFromFields(fields)).toEqual({ name: '', url: '' })
  })

  it('coerces numeric and string defaults to strings', () => {
    const fields: ModalField[] = [
      { name: 'priority', label: 'Priority', type: 'number', defaultValue: 0 },
      { name: 'mode', label: 'Mode', type: 'select', defaultValue: 'bearer' },
    ]
    expect(initialValuesFromFields(fields)).toEqual({ priority: '0', mode: 'bearer' })
  })
})

describe('groupKeyPoolMappings', () => {
  it('groups flat (api_key, provider_key) pairs by (api_key, provider)', () => {
    const providerKeys = {
      'prov-A': [pk('pk1', 'prov-A'), pk('pk2', 'prov-A')],
      'prov-B': [pk('pk3', 'prov-B')],
    }
    const mappings: KeyPoolMappingView[] = [
      { api_key_id: 'ak1', provider_key_id: 'pk1' },
      { api_key_id: 'ak1', provider_key_id: 'pk2' },
      { api_key_id: 'ak1', provider_key_id: 'pk3' },
      { api_key_id: 'ak2', provider_key_id: 'pk1' },
    ]
    const out = groupKeyPoolMappings(mappings, providerKeys)
    // Three buckets: (ak1,prov-A) (ak1,prov-B) (ak2,prov-A)
    expect(out).toHaveLength(3)
    const ak1A = out.find((r) => r.apiKeyId === 'ak1' && r.providerId === 'prov-A')
    expect(ak1A?.providerKeyIds.sort()).toEqual(['pk1', 'pk2'])
    const ak1B = out.find((r) => r.apiKeyId === 'ak1' && r.providerId === 'prov-B')
    expect(ak1B?.providerKeyIds).toEqual(['pk3'])
    const ak2A = out.find((r) => r.apiKeyId === 'ak2' && r.providerId === 'prov-A')
    expect(ak2A?.providerKeyIds).toEqual(['pk1'])
  })

  it('drops mappings whose provider_key_id is unknown (deleted out-of-band)', () => {
    const providerKeys = { 'prov-A': [pk('pk1', 'prov-A')] }
    const mappings: KeyPoolMappingView[] = [
      { api_key_id: 'ak1', provider_key_id: 'pk1' },
      { api_key_id: 'ak1', provider_key_id: 'ghost' },
    ]
    const out = groupKeyPoolMappings(mappings, providerKeys)
    expect(out).toHaveLength(1)
    expect(out[0].providerKeyIds).toEqual(['pk1'])
  })

  it('returns an empty list when there are no mappings', () => {
    expect(groupKeyPoolMappings([], {})).toEqual([])
  })
})
