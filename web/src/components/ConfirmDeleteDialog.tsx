import React, { useState } from 'react'

interface Props {
  title: string
  message: React.ReactNode
  confirmText: string
  /** require user to type this to enable the destructive button */
  typedConfirmation?: string
  onCancel: () => void
  onConfirm: () => Promise<void>
}

export const ConfirmDeleteDialog: React.FC<Props> = ({
  title,
  message,
  confirmText,
  typedConfirmation,
  onCancel,
  onConfirm,
}) => {
  const [typed, setTyped] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const canConfirm = !typedConfirmation || typed === typedConfirmation

  const click = async () => {
    if (!canConfirm) return
    setError(null)
    setBusy(true)
    try {
      await onConfirm()
    } catch (e: any) {
      setError(e?.response?.data?.message || e?.message || 'Request failed')
      setBusy(false)
      return
    }
    setBusy(false)
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-md p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-3">{title}</h2>
        <div className="text-sm text-gray-700 mb-4">{message}</div>
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded mb-3 text-sm">
            {error}
          </div>
        )}
        {typedConfirmation && (
          <label className="block text-sm mb-4">
            <span className="text-gray-700">
              Type <code className="font-mono bg-gray-100 px-1 rounded">{typedConfirmation}</code>{' '}
              to confirm
            </span>
            <input
              type="text"
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-red-500"
            />
          </label>
        )}
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-100 rounded-md transition"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={click}
            disabled={!canConfirm || busy}
            className="px-4 py-1.5 text-sm bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-40 transition"
          >
            {busy ? 'Deleting…' : confirmText}
          </button>
        </div>
      </div>
    </div>
  )
}
