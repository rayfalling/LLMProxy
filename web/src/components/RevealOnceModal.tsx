import React, { useState } from 'react'

interface Props {
  title: string
  secret: string
  helpText?: string
  onClose: () => void
}

/** Modal that shows a freshly minted secret exactly once.
 *  The user must explicitly acknowledge the warning to dismiss it. */
export const RevealOnceModal: React.FC<Props> = ({ title, secret, helpText, onClose }) => {
  const [copied, setCopied] = useState(false)

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(secret)
      setCopied(true)
    } catch {
      // Clipboard may be unavailable in non-secure contexts; selection still works.
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-lg p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-2">{title}</h2>
        <p className="text-sm text-amber-700 bg-amber-50 border border-amber-200 rounded px-3 py-2 mb-4">
          This is the only time the full key will be shown. Save it somewhere safe before closing
          this dialog.
        </p>
        <div className="bg-gray-50 border border-gray-200 rounded px-3 py-2 font-mono text-sm break-all select-all">
          {secret}
        </div>
        {helpText && <div className="text-xs text-gray-500 mt-2">{helpText}</div>}
        <div className="mt-5 flex justify-end gap-2">
          <button
            type="button"
            onClick={copy}
            className="px-3 py-1.5 text-sm bg-gray-100 text-gray-800 rounded-md hover:bg-gray-200 transition"
          >
            {copied ? 'Copied!' : 'Copy to clipboard'}
          </button>
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-1.5 text-sm bg-indigo-600 text-white rounded-md hover:bg-indigo-700 transition"
          >
            I have saved the key
          </button>
        </div>
      </div>
    </div>
  )
}
