import { useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import NoteList from '../components/note-list.tsx'
import ProjectForm from '../components/project-form.tsx'
import { errorMessage } from '../notes.ts'
import { useDeleteProject, useProject, useUpdateProject } from '../projects.ts'
import NotFound from './not-found.tsx'

export default function ProjectPage() {
  const id = Number(useParams().id)
  return Number.isInteger(id) ? <ProjectDetail key={id} id={id} /> : <NotFound />
}

function ProjectDetail({ id }: { id: number }) {
  const navigate = useNavigate()
  const [editing, setEditing] = useState(false)
  const project = useProject(id)
  const update = useUpdateProject()
  const remove = useDeleteProject()
  const params = { params: { path: { id } } }

  if (project.isPending) return <p>Loading…</p>
  if (project.isError) {
    return project.error.error === 'project not found' ? (
      <NotFound />
    ) : (
      <p className="error">Failed to load project: {errorMessage(project.error)}</p>
    )
  }

  return (
    <>
      <p>
        <Link to="/projects">← Projects</Link>
      </p>
      {editing ? (
        <ProjectForm
          initial={project.data}
          submitLabel="Save"
          pending={update.isPending}
          error={update.error}
          onSubmit={(body) => update.mutate({ ...params, body }, { onSuccess: () => setEditing(false) })}
          onCancel={() => {
            update.reset()
            setEditing(false)
          }}
        />
      ) : (
        <>
          <h1>{project.data.name}</h1>
          {project.data.description && <p className="note-body">{project.data.description}</p>}
          <div className="actions">
            <button type="button" onClick={() => setEditing(true)}>
              Edit
            </button>
            <button
              type="button"
              disabled={remove.isPending}
              onClick={() => {
                if (confirm(`Delete “${project.data.name}”? Its notes will be kept, without a project.`)) {
                  remove.mutate(params, { onSuccess: () => navigate('/projects') })
                }
              }}
            >
              Delete
            </button>
            {remove.isError && <span className="error">{errorMessage(remove.error)}</span>}
          </div>
        </>
      )}
      <h2>Notes</h2>
      <NoteList projectId={id} />
    </>
  )
}
