import createFetchClient from 'openapi-fetch'
import createClient from 'openapi-react-query'
import type { components, paths } from './api.gen.ts'

// Types and paths come from api.gen.ts, generated from the server's OpenAPI document
// (../openapi.json). Run `npm run gen:api` after changing the API.
export const fetchClient = createFetchClient<paths>()

// Typed TanStack Query hooks, e.g. `api.useQuery('get', '/api/health')`.
export const api = createClient(fetchClient)

export type Health = components['schemas']['Health']
export type Note = components['schemas']['Note']
export type NoteInput = components['schemas']['NoteInput']
export type ErrorBody = components['schemas']['ErrorBody']
export type Project = components['schemas']['Project']
export type ProjectInput = components['schemas']['ProjectInput']
