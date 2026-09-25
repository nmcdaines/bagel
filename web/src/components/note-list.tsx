import { useState, type FormEvent } from 'react'
import { Link } from 'react-router'
import type { Note, Project } from '../api.ts'
import {
  errorMessage,
  useCreateNote,
  useDeleteNote,
  useNotes,
  useUpdateNote,
} from '../notes.ts'
import { useProjects } from '../projects.ts'

/** Notes with a form to add one. Given a `projectId`, shows and adds only that project's notes. */
export default function NoteList({ projectId }: { projectId?: number }) {
  const notes = useNotes(projectId)
  const projects = useProjects()

  return (
    <>
      <CreateNoteForm projectId={projectId} projects={projects.data ?? []} />
      {notes.isPending ? (
        <p>Loading…</p>
      ) : notes.isError ? (
        <p className="error">Failed to load notes: {errorMessage(notes.error)}</p>
      ) : notes.data.length === 0 ? (
        <p>No notes yet.</p>
      ) : (
        <ul className="notes">
          {notes.data.map((note) => (
            <NoteItem
              key={note.id}
              note={note}
              projects={projects.data ?? []}
              showProject={projectId === undefined}
            />
          ))}
        </ul>
      )}
    </>
  )
}

function ProjectSelect({
  projects,
  value,
  onChange,
}: {
  projects: Project[]
  value: number | null
  onChange: (id: number | null) => void
}) {
  return (
    <select
      aria-label="Project"
      value={value ?? ''}
      onChange={(e) => onChange(e.target.value ? Number(e.target.value) : null)}
    >
      <option value="">No project</option>
      {projects.map((p) => (
        <option key={p.id} value={p.id}>
          {p.name}
        </option>
      ))}
    </select>
  )
}

function CreateNoteForm({ projectId, projects }: { projectId?: number; projects: Project[] }) {
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [chosenProject, setChosenProject] = useState<number | null>(null)
  const create = useCreateNote()

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    create.mutate(
      { body: { title, body, project_id: projectId ?? chosenProject } },
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
      {projectId === undefined && (
        <ProjectSelect projects={projects} value={chosenProject} onChange={setChosenProject} />
      )}
      <div className="actions">
        <button type="submit" disabled={create.isPending || !title.trim()}>
          Add note
        </button>
        {create.isError && <span className="error">{errorMessage(create.error)}</span>}
      </div>
    </form>
  )
}

function NoteItem({
  note,
  projects,
  showProject,
}: {
  note: Note
  projects: Project[]
  showProject: boolean
}) {
  const [editing, setEditing] = useState(false)
  const [title, setTitle] = useState(note.title)
  const [body, setBody] = useState(note.body)
  const [projectId, setProjectId] = useState(note.project_id ?? null)
  const update = useUpdateNote()
  const remove = useDeleteNote()
  const project = projects.find((p) => p.id === note.project_id)

  function startEditing() {
    setTitle(note.title)
    setBody(note.body)
    setProjectId(note.project_id ?? null)
    update.reset()
    setEditing(true)
  }

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    update.mutate(
      { params: { path: { id: note.id } }, body: { title, body, project_id: projectId } },
      { onSuccess: () => setEditing(false) },
    )
  }

  if (editing) {
    return (
      <li>
        <form className="note-form" onSubmit={onSubmit}>
          <input value={title} onChange={(e) => setTitle(e.target.value)} required />
          <textarea rows={3} value={body} onChange={(e) => setBody(e.target.value)} />
          <ProjectSelect projects={projects} value={projectId} onChange={setProjectId} />
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
      {showProject && project && (
        <Link className="note-project" to={`/projects/${project.id}`}>
          {project.name}
        </Link>
      )}
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
