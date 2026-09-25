import createClient from 'openapi-fetch'
import type { components, paths } from './api.gen.ts'

// Types and paths come from api.gen.ts, generated from the server's OpenAPI document
// (../openapi.json). Run `npm run gen:api` after changing the API.
export const api = createClient<paths>()

export type Health = components['schemas']['Health']

export async function fetchHealth(): Promise<Health | null> {
  try {
    const { data } = await api.GET('/api/health')
    return data ?? null
  } catch {
    return null
  }
}
