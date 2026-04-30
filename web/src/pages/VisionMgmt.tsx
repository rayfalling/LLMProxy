import React, { useEffect, useState } from 'react'
import { AppLayout } from '../components/AppLayout'
import { ResourceCreateModal, ModalField } from '../components/ResourceCreateModal'
import { ConfirmDeleteDialog } from '../components/ConfirmDeleteDialog'
import { apiClient } from '../api/client'
import { VisionMappingView } from '../api/types'

export const VisionMgmt: React.FC = () => {
  const [rows, setRows] = useState<VisionMappingView[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [edits, setEdits] = useState<Record<string, { vision: string; gen: string }>>({})
  const [savingModel, setSavingModel] = useState<string | null>(null)
  const [showCreate, setShowCreate] = useState(false)
  const [pendingDelete, setPendingDelete] = useState<VisionMappingView | null>(null)

  const fields: ModalField[] = [
    { name: 'model_name', label: 'Model name', required: true, placeholder: 'e.g. gpt-4o-vision' },
    { name: 'vision_parser_alias', label: 'Vision parser alias (optional)' },
    { name: 'generation_alias', label: 'Generation alias (optional)' },
  ]

  const load = async () => {
    try {
      const list = await apiClient.listVisionMappings()
      setRows(list)
      const e: Record<string, { vision: string; gen: string }> = {}
      list.forEach(
        (r) =>
          (e[r.model_name] = {
            vision: r.vision_parser_alias || '',
            gen: r.generation_alias || '',
          }),
      )
      setEdits(e)
    } catch (err: any) {
      setError(err.response?.data?.message || 'Failed to load vision mappings')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    load()
  }, [])

  const onSave = async (modelName: string) => {
    setSavingModel(modelName)
    try {
      const v = edits[modelName] || { vision: '', gen: '' }
      await apiClient.updateVisionMapping(
        modelName,
        v.vision.trim() || null,
        v.gen.trim() || null,
      )
      await load()
    } catch (e: any) {
      setError(e.response?.data?.message || 'Failed to update mapping')
    } finally {
      setSavingModel(null)
    }
  }

  return (
    <AppLayout>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900">Vision mappings</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="px-3 py-1.5 text-sm bg-indigo-600 text-white rounded-md hover:bg-indigo-700 transition"
        >
          + Add mapping
        </button>
      </div>
      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4">
          {error}
        </div>
      )}
      {loading ? (
        <div className="text-gray-500">Loading…</div>
      ) : (
        <div className="bg-white rounded-lg shadow overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-gray-50 text-gray-600">
              <tr>
                <th className="px-6 py-3 text-left">Model</th>
                <th className="px-6 py-3 text-left">Vision parser alias</th>
                <th className="px-6 py-3 text-left">Generation alias</th>
                <th className="px-6 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {rows.map((r) => {
                const e = edits[r.model_name] || { vision: '', gen: '' }
                const dirty =
                  e.vision !== (r.vision_parser_alias || '') ||
                  e.gen !== (r.generation_alias || '')
                return (
                  <tr key={r.model_name}>
                    <td className="px-6 py-3 font-mono text-xs">{r.model_name}</td>
                    <td className="px-6 py-3">
                      <input
                        type="text"
                        value={e.vision}
                        onChange={(ev) =>
                          setEdits((prev) => ({
                            ...prev,
                            [r.model_name]: { ...e, vision: ev.target.value },
                          }))
                        }
                        className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                        placeholder="(none)"
                      />
                    </td>
                    <td className="px-6 py-3">
                      <input
                        type="text"
                        value={e.gen}
                        onChange={(ev) =>
                          setEdits((prev) => ({
                            ...prev,
                            [r.model_name]: { ...e, gen: ev.target.value },
                          }))
                        }
                        className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                        placeholder="(none)"
                      />
                    </td>
                    <td className="px-6 py-3 text-right">
                      <button
                        disabled={!dirty || savingModel === r.model_name}
                        onClick={() => onSave(r.model_name)}
                        className="px-3 py-1 text-sm rounded bg-indigo-600 text-white disabled:bg-gray-300 disabled:cursor-not-allowed mr-2"
                      >
                        {savingModel === r.model_name ? 'Saving…' : 'Save'}
                      </button>
                      <button
                        onClick={() => setPendingDelete(r)}
                        className="text-red-600 hover:text-red-800 text-sm font-medium"
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
          {rows.length === 0 && (
            <div className="px-6 py-8 text-center text-gray-500">No vision mappings.</div>
          )}
        </div>
      )}
      {showCreate && (
        <ResourceCreateModal
          title="Add vision mapping"
          fields={fields}
          onCancel={() => setShowCreate(false)}
          onSubmit={async (v) => {
            await apiClient.createVisionMapping({
              model_name: v.model_name,
              vision_parser_alias: v.vision_parser_alias || null,
              generation_alias: v.generation_alias || null,
            })
            setShowCreate(false)
            await load()
          }}
        />
      )}
      {pendingDelete && (
        <ConfirmDeleteDialog
          title="Delete vision mapping"
          message={
            <span>
              Remove vision mapping for <code>{pendingDelete.model_name}</code>?
            </span>
          }
          confirmText="Delete"
          onCancel={() => setPendingDelete(null)}
          onConfirm={async () => {
            await apiClient.deleteVisionMapping(pendingDelete.model_name)
            setPendingDelete(null)
            await load()
          }}
        />
      )}
    </AppLayout>
  )
}
