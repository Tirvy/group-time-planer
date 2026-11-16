/** Health endpoint payload from the Rust backend. */
export interface HealthResponse {
  status: 'ok' | 'degraded'
  service: string
  version: string
  database: 'ok' | 'error'
}

export function isHealthResponse(value: unknown): value is HealthResponse {
  if (typeof value !== 'object' || value === null) return false
  const candidate = value as Record<string, unknown>
  return (
    (candidate.status === 'ok' || candidate.status === 'degraded') &&
    typeof candidate.service === 'string' &&
    typeof candidate.version === 'string' &&
    (candidate.database === 'ok' || candidate.database === 'error')
  )
}
