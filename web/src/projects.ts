import { useQueryClient } from '@tanstack/react-query'
import { api } from './api.ts'

const listKey = api.queryOptions('get', '/api/projects').queryKey
const detailKey = (id: number) =>
  api.queryOptions('get', '/api/projects/{id}', { params: { path: { id } } }).queryKey
// Prefix of every notes list query, filtered or not.
const notesKey = api.queryOptions('get', '/api/notes').queryKey

export function useProjects() {
  return api.useQuery('get', '/api/projects')
}

export function useProject(id: number) {
  return api.useQuery('get', '/api/projects/{id}', { params: { path: { id } } })
}

export function useCreateProject() {
  const queryClient = useQueryClient()
  return api.useMutation('post', '/api/projects', {
    onSuccess: () => queryClient.invalidateQueries({ queryKey: listKey }),
  })
}

export function useUpdateProject() {
  const queryClient = useQueryClient()
  return api.useMutation('put', '/api/projects/{id}', {
    onSuccess: (project) => {
      queryClient.setQueryData(detailKey(project.id), project)
      return queryClient.invalidateQueries({ queryKey: listKey })
    },
  })
}

export function useDeleteProject() {
  const queryClient = useQueryClient()
  return api.useMutation('delete', '/api/projects/{id}', {
    onSuccess: (_data, { params: { path: { id } } }) => {
      queryClient.removeQueries({ queryKey: detailKey(id) })
      // The project's notes are kept but unassigned, so notes lists change too.
      return Promise.all([
        queryClient.invalidateQueries({ queryKey: listKey }),
        queryClient.invalidateQueries({ queryKey: notesKey }),
      ])
    },
  })
}
