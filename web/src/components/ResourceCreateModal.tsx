import React, { useState } from 'react'

export interface ModalField {
  name: string
  label: string
  type?: 'text' | 'password' | 'number' | 'select' | 'url'
  required?: boolean
  placeholder?: string
  options?: { value: string; label: string }[]
  helpText?: string
  defaultValue?: string | number
}

interface Props {
  title: string
  fields: ModalField[]
  submitLabel?: string
  onCancel: () => void
  onSubmit: (values: Record<string, string>) => Promise<void>
}

/** Generic modal for creating a resource — renders a configurable form,
 *  surfaces server-side validation errors, and exposes a single submit handler. */
export const ResourceCreateModal: React.FC<Props> = ({
  title,
  fields,
  submitLabel,
  onCancel,
  onSubmit,
}) => {
  const initial: Record<string, string> = {}
  for (const f of fields) {
    initial[f.name] =
      f.defaultValue !== undefined ? String(f.defaultValue) : ''
  }
  const [values, setValues] = useState<Record<string, string>>(initial)
  const [error, setError] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setSubmitting(true)
    try {
      await onSubmit(values)
    } catch (err: any) {
      setError(err?.response?.data?.message || err?.message || 'Request failed')
      setSubmitting(false)
      return
    }
    setSubmitting(false)
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <form
        onSubmit={handleSubmit}
        className="bg-white rounded-lg shadow-xl w-full max-w-md p-6"
      >
        <h2 className="text-lg font-semibold text-gray-900 mb-4">{title}</h2>
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded mb-3 text-sm">
            {error}
          </div>
        )}
        <div className="space-y-3">
          {fields.map((f) => (
            <label key={f.name} className="block text-sm">
              <span className="text-gray-700">
                {f.label}
                {f.required && <span className="text-red-500 ml-0.5">*</span>}
              </span>
              {f.type === 'select' ? (
                <select
                  value={values[f.name]}
                  onChange={(e) =>
                    setValues((v) => ({ ...v, [f.name]: e.target.value }))
                  }
                  required={f.required}
                  className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                >
                  {!f.required && <option value="">(none)</option>}
                  {f.options?.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  type={f.type ?? 'text'}
                  value={values[f.name]}
                  onChange={(e) =>
                    setValues((v) => ({ ...v, [f.name]: e.target.value }))
                  }
                  placeholder={f.placeholder}
                  required={f.required}
                  className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-indigo-500"
                />
              )}
              {f.helpText && (
                <span className="text-xs text-gray-500 mt-0.5 block">{f.helpText}</span>
              )}
            </label>
          ))}
        </div>
        <div className="mt-5 flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-100 rounded-md transition"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={submitting}
            className="px-4 py-1.5 text-sm bg-indigo-600 text-white rounded-md hover:bg-indigo-700 disabled:opacity-50 transition"
          >
            {submitting ? 'Submitting…' : submitLabel ?? 'Create'}
          </button>
        </div>
      </form>
    </div>
  )
}
