import { useQueryClient } from '@tanstack/react-query'
import { api, type Note } from './api.ts'

// Query keys come from openapi-react-query (`[method, path, init]`), so they stay in sync with
// the paths in the OpenAPI contract. `listKey` has no params, so it prefixes every notes list,
// including those filtered by project.
const listKey = api.queryOptions('get', '/api/notes').queryKey
const detailKey = (id: number) =>
  api.queryOptions('get', '/api/notes/{id}', { params: { path: { id } } }).queryKey

/** Best-effort message for an error thrown by the API client. */
export function errorMessage(error: unknown): string {
  if (error && typeof error === 'object' && 'error' in error && typeof error.error === 'string') {
    return error.error
  }
  return error instanceof Error ? error.message : 'Request failed'
}

/** All notes, or only those in `projectId` when given. */
export function useNotes(projectId?: number) {
  return api.useQuery(
    'get',
    '/api/notes',
    projectId === undefined ? undefined : { params: { query: { project_id: projectId } } },
  )
}

export function useCreateNote() {
  const queryClient = useQueryClient()
  return api.useMutation('post', '/api/notes', {
    onSuccess: () => queryClient.invalidateQueries({ queryKey: listKey }),
  })
}

export function useUpdateNote() {
  const queryClient = useQueryClient()
  return api.useMutation('put', '/api/notes/{id}', {
    onSuccess: (note) => {
      queryClient.setQueryData(detailKey(note.id), note)
      return queryClient.invalidateQueries({ queryKey: listKey })
    },
  })
}

export function useDeleteNote() {
  const queryClient = useQueryClient()
  return api.useMutation('delete', '/api/notes/{id}', {
    // Optimistically drop the note from every list; roll back if the request fails.
    onMutate: async ({ params: { path: { id } } }) => {
      await queryClient.cancelQueries({ queryKey: listKey })
      const previous = queryClient.getQueriesData<Note[]>({ queryKey: listKey })
      queryClient.setQueriesData<Note[]>({ queryKey: listKey }, (notes) =>
        notes?.filter((n) => n.id !== id),
      )
      return { previous }
    },
    onError: (_err, _vars, context) => {
      for (const [key, notes] of context?.previous ?? []) queryClient.setQueryData(key, notes)
    },
    onSettled: (_data, _err, { params: { path: { id } } }) => {
      queryClient.removeQueries({ queryKey: detailKey(id) })
      return queryClient.invalidateQueries({ queryKey: listKey })
    },
  })
}
