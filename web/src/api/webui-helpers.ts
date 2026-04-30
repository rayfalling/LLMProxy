import { ModalField } from '../components/ResourceCreateModal'
import { KeyPoolMappingView, ProviderKeyView } from './types'

/** Build the initial form-value map for a ResourceCreateModal from its field
 *  descriptors. Pure helper — extracted so it can be tested without rendering. */
export function initialValuesFromFields(fields: ModalField[]): Record<string, string> {
  const out: Record<string, string> = {}
  for (const f of fields) {
    out[f.name] = f.defaultValue !== undefined ? String(f.defaultValue) : ''
  }
  return out
}

export interface GroupedKeyPoolRow {
  apiKeyId: string
  providerId: string
  providerKeyIds: string[]
}

/** Group flat (api_key_id, provider_key_id) rows into per-(api_key, provider)
 *  buckets by joining each provider_key_id back to its owning provider via the
 *  per-provider provider-key listing. Mappings whose provider key is missing
 *  from `providerKeys` (e.g. deleted out-of-band) are silently skipped. */
export function groupKeyPoolMappings(
  mappings: KeyPoolMappingView[],
  providerKeys: Record<string, ProviderKeyView[]>,
): GroupedKeyPoolRow[] {
  const lookup = new Map<string, string>()
  for (const [provId, list] of Object.entries(providerKeys)) {
    for (const k of list) lookup.set(k.id, provId)
  }
  const rows = new Map<string, GroupedKeyPoolRow>()
  for (const m of mappings) {
    const provId = lookup.get(m.provider_key_id)
    if (!provId) continue
    const k = `${m.api_key_id}__${provId}`
    const existing = rows.get(k)
    if (existing) {
      existing.providerKeyIds.push(m.provider_key_id)
    } else {
      rows.set(k, {
        apiKeyId: m.api_key_id,
        providerId: provId,
        providerKeyIds: [m.provider_key_id],
      })
    }
  }
  return Array.from(rows.values())
}
