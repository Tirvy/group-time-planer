import { isHealthResponse, type HealthResponse } from '../types/api'

interface ErrorEnvelope {
  error?: {
    code?: string
    message?: string
  }
}

/** Stable, machine-readable API failure. The UI maps `code` to Russian copy. */
export class ApiError extends Error {
  readonly code: string
  readonly status: number | null

  constructor(code: string, message: string, status: number | null = null) {
    super(message)
    this.name = 'ApiError'
    this.code = code
    this.status = status
  }
}

export async function fetchHealth(signal?: AbortSignal): Promise<HealthResponse> {
  let response: Response
  try {
    response = await fetch('/health', {
      headers: { Accept: 'application/json' },
      signal,
    })
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === 'AbortError') throw cause
    throw new ApiError('NETWORK_ERROR', 'health request failed')
  }

  const body = await readJson(response)

  // A 503 still carries the structured health payload, which lets the UI show a
  // degraded state instead of a generic failure.
  if (response.status === 503 && isHealthResponse(body) && body.status === 'degraded') {
    return body
  }

  if (!response.ok) {
    const envelope = body as ErrorEnvelope | null
    throw new ApiError(
      envelope?.error?.code ?? 'HTTP_ERROR',
      envelope?.error?.message ?? `health request failed with ${response.status}`,
      response.status,
    )
  }

  if (!isHealthResponse(body)) {
    throw new ApiError('HTTP_ERROR', 'unexpected health response shape', response.status)
  }

  return body
}

async function readJson(response: Response): Promise<unknown> {
  try {
    return await response.json()
  } catch {
    return null
  }
}
