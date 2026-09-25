import { useState, type FormEvent } from 'react'
import type { ProjectInput } from '../api.ts'
import { errorMessage } from '../notes.ts'

type Props = {
  initial?: ProjectInput
  submitLabel: string
  pending: boolean
  error: unknown
  onSubmit: (input: ProjectInput, reset: () => void) => void
  onCancel?: () => void
}

export default function ProjectForm({ initial, submitLabel, pending, error, onSubmit, onCancel }: Props) {
  const [name, setName] = useState(initial?.name ?? '')
  const [description, setDescription] = useState(initial?.description ?? '')

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    onSubmit({ name, description }, () => {
      setName('')
      setDescription('')
    })
  }

  return (
    <form className="note-form" onSubmit={handleSubmit}>
      <input placeholder="Name" value={name} onChange={(e) => setName(e.target.value)} required />
      <textarea
        placeholder="Description (optional)"
        rows={2}
        value={description}
        onChange={(e) => setDescription(e.target.value)}
      />
      <div className="actions">
        <button type="submit" disabled={pending || !name.trim()}>
          {submitLabel}
        </button>
        {onCancel && (
          <button type="button" onClick={onCancel}>
            Cancel
          </button>
        )}
        {error != null && <span className="error">{errorMessage(error)}</span>}
      </div>
    </form>
  )
}
