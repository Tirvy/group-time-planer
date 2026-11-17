import { afterEach, describe, expect, it, vi } from 'vitest'

import { ApiError, authenticate, fetchBestTime, fetchTimetablePng } from './client'

const fetchMock = vi.fn()

vi.stubGlobal('fetch', fetchMock)

afterEach(() => {
  fetchMock.mockReset()
})

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

function requestInit(call = 0): RequestInit {
  return (fetchMock.mock.calls[call]?.[1] ?? {}) as RequestInit
}

function requestHeaders(call = 0): Record<string, string> {
  return (requestInit(call).headers ?? {}) as Record<string, string>
}

describe('api client', () => {
  it('exchanges initData for the user profile', async () => {
    fetchMock.mockResolvedValue(
      jsonResponse({ user: { telegram_id: '42', timezone_set: false } }),
    )

    const response = await authenticate('query_id=abc&hash=def')

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/auth/telegram',
      expect.objectContaining({ method: 'POST' }),
    )
    expect(requestInit().body).toBe(
      JSON.stringify({ initData: 'query_id=abc&hash=def' }),
    )
    expect(response.user.telegram_id).toBe('42')
  })

  it('sends validated initData as the tma authorization header', async () => {
    fetchMock.mockResolvedValue(jsonResponse({ intervals: [] }))

    await fetchBestTime('signed-init-data', '7')

    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/groups/7/best-time')
    expect(requestHeaders().Authorization).toBe('tma signed-init-data')
  })

  it('maps error envelopes to a stable ApiError', async () => {
    fetchMock.mockResolvedValue(
      jsonResponse(
        { error: { code: 'NOT_A_MEMBER', message: 'no access' } },
        403,
      ),
    )

    await expect(fetchBestTime('init', '7')).rejects.toMatchObject({
      name: 'ApiError',
      code: 'NOT_A_MEMBER',
      status: 403,
    })
  })

  it('reports network failures instead of leaking fetch errors', async () => {
    fetchMock.mockRejectedValue(new TypeError('failed to fetch'))

    const error = await fetchBestTime('init', '7').catch((caught) => caught)

    expect(error).toBeInstanceOf(ApiError)
    expect(error.code).toBe('NETWORK_ERROR')
    expect(error.status).toBeNull()
  })

  it('returns the timetable blob and maps its error envelope', async () => {
    fetchMock.mockResolvedValueOnce({
      ok: true,
      blob: async () => new Blob(['png']),
    })
    const blob = await fetchTimetablePng('init', '7')
    expect(requestHeaders().Accept).toBe('image/png')
    expect(await blob.text()).toBe('png')

    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: { code: 'TIMEZONE_REQUIRED' } }, 409),
    )
    await expect(fetchTimetablePng('init', '7')).rejects.toMatchObject({
      code: 'TIMEZONE_REQUIRED',
      status: 409,
    })
  })
})
