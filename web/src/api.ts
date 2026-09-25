export type Health = { status: string; version: string }

export async function fetchHealth(): Promise<Health | null> {
  try {
    const res = await fetch('/api/health')
    return res.ok ? await res.json() : null
  } catch {
    return null
  }
}
