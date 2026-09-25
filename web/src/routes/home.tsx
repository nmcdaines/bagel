import { api } from '../api.ts'

export default function Home() {
  const { data: health, isPending } = api.useQuery('get', '/api/health')

  return (
    <>
      <h1>Welcome to bagel</h1>
      <p>
        API:{' '}
        {isPending
          ? 'checking…'
          : health
            ? `${health.status} (v${health.version})`
            : 'unreachable'}
      </p>
    </>
  )
}
