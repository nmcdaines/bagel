import { useState, type FormEvent } from 'react'
import type { Note } from '../api.ts'
import {
  errorMessage,
  useCreateNote,
  useDeleteNote,
  useNotes,
  useUpdateNote,
} from '../notes.ts'

export default function Notes() {
  const notes = useNotes()

  return (
    <>
      <h1>Notes</h1>
      <CreateNoteForm />
      {notes.isPending ? (
        <p>Loading…</p>
      ) : notes.isError ? (
        <p className="error">Failed to load notes: {errorMessage(notes.error)}</p>
      ) : notes.data.length === 0 ? (
        <p>No notes yet.</p>
      ) : (
        <ul className="notes">
          {notes.data.map((note) => (
            <NoteItem key={note.id} note={note} />
          ))}
        </ul>
      )}
    </>
  )
}

function CreateNoteForm() {
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const create = useCreateNote()

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    create.mutate(
      { body: { title, body } },
      {
        onSuccess: () => {
          setTitle('')
          setBody('')
        },
      },
    )
  }

  return (
    <form className="note-form" onSubmit={onSubmit}>
      <input
        placeholder="Title"
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        required
      />
      <textarea
        placeholder="Body"
        rows={3}
        value={body}
        onChange={(e) => setBody(e.target.value)}
      />
      <div className="actions">
        <button type="submit" disabled={create.isPending || !title.trim()}>
          Add note
        </button>
        {create.isError && <span className="error">{errorMessage(create.error)}</span>}
      </div>
    </form>
  )
}

function NoteItem({ note }: { note: Note }) {
  const [editing, setEditing] = useState(false)
  const [title, setTitle] = useState(note.title)
  const [body, setBody] = useState(note.body)
  const update = useUpdateNote()
  const remove = useDeleteNote()

  function startEditing() {
    setTitle(note.title)
    setBody(note.body)
    update.reset()
    setEditing(true)
  }

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    update.mutate(
      { params: { path: { id: note.id } }, body: { title, body } },
      { onSuccess: () => setEditing(false) },
    )
  }

  if (editing) {
    return (
      <li>
        <form className="note-form" onSubmit={onSubmit}>
          <input value={title} onChange={(e) => setTitle(e.target.value)} required />
          <textarea rows={3} value={body} onChange={(e) => setBody(e.target.value)} />
          <div className="actions">
            <button type="submit" disabled={update.isPending || !title.trim()}>
              Save
            </button>
            <button type="button" onClick={() => setEditing(false)}>
              Cancel
            </button>
            {update.isError && <span className="error">{errorMessage(update.error)}</span>}
          </div>
        </form>
      </li>
    )
  }

  return (
    <li>
      <h2>{note.title}</h2>
      {note.body && <p className="note-body">{note.body}</p>}
      <div className="actions">
        <small>Updated {new Date(note.updated_at).toLocaleString()}</small>
        <button type="button" onClick={startEditing}>
          Edit
        </button>
        <button type="button" onClick={() => remove.mutate({ params: { path: { id: note.id } } })} disabled={remove.isPending}>
          Delete
        </button>
        {remove.isError && <span className="error">{errorMessage(remove.error)}</span>}
      </div>
    </li>
  )
}
