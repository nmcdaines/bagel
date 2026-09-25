import { useLoaderData } from 'react-router'
import type { Health } from '../api.ts'

export default function Home() {
  const health = useLoaderData<Health | null>()

  return (
    <>
      <h1>Welcome to bagel</h1>
      <p>
        API:{' '}
        {health ? `${health.status} (v${health.version})` : 'unreachable'}
      </p>
    </>
  )
}
