import { useQueryClient } from '@tanstack/react-query'
import { api, type Note } from './api.ts'

// Query keys come from openapi-react-query (`[method, path, init]`), so they stay in sync with
// the paths in the OpenAPI contract.
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

export function useNotes() {
  return api.useQuery('get', '/api/notes')
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
    // Optimistically drop the note from the list; roll back if the request fails.
    onMutate: async ({ params: { path: { id } } }) => {
      await queryClient.cancelQueries({ queryKey: listKey })
      const previous = queryClient.getQueryData<Note[]>(listKey)
      queryClient.setQueryData<Note[]>(listKey, (notes) => notes?.filter((n) => n.id !== id))
      return { previous }
    },
    onError: (_err, _vars, context) => {
      queryClient.setQueryData(listKey, context?.previous)
    },
    onSettled: (_data, _err, { params: { path: { id } } }) => {
      queryClient.removeQueries({ queryKey: detailKey(id) })
      return queryClient.invalidateQueries({ queryKey: listKey })
    },
  })
}
