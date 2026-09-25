import { Link } from 'react-router'
import ProjectForm from '../components/project-form.tsx'
import { errorMessage } from '../notes.ts'
import { useCreateProject, useProjects } from '../projects.ts'

export default function Projects() {
  const projects = useProjects()
  const create = useCreateProject()

  return (
    <>
      <h1>Projects</h1>
      <ProjectForm
        submitLabel="Add project"
        pending={create.isPending}
        error={create.error}
        onSubmit={(body, reset) => create.mutate({ body }, { onSuccess: reset })}
      />
      {projects.isPending ? (
        <p>Loading…</p>
      ) : projects.isError ? (
        <p className="error">Failed to load projects: {errorMessage(projects.error)}</p>
      ) : projects.data.length === 0 ? (
        <p>No projects yet.</p>
      ) : (
        <ul className="notes">
          {projects.data.map((project) => (
            <li key={project.id}>
              <h2>
                <Link to={`/projects/${project.id}`}>{project.name}</Link>
              </h2>
              {project.description && <p className="note-body">{project.description}</p>}
            </li>
          ))}
        </ul>
      )}
    </>
  )
}
