import type {
  GroupsResponse,
  GroupCalendarResponse,
  MeResponse,
  MemberCalendarResponse,
  PreferenceInterval,
  PreferencesResponse,
} from '../types/api'

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

/** Exchanges raw Telegram initData for the authenticated user profile. */
export async function authenticate(initData: string): Promise<MeResponse> {
  return request<MeResponse>('/api/auth/telegram', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ initData }),
  })
}

/** Re-reads the user profile. Identity always comes from the auth header. */
export async function fetchMe(initData: string): Promise<MeResponse> {
  return request<MeResponse>('/api/me', {
    headers: authHeaders(initData),
  })
}

export async function updateTimezone(
  initData: string,
  timezone: string,
): Promise<MeResponse> {
  return request<MeResponse>('/api/me/timezone', {
    method: 'PUT',
    headers: {
      ...authHeaders(initData),
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ timezone }),
  })
}

export async function fetchGroups(initData: string): Promise<GroupsResponse> {
  return request<GroupsResponse>('/api/groups', {
    headers: authHeaders(initData),
  })
}

export async function fetchPreferences(
  initData: string,
  groupId: string,
): Promise<PreferencesResponse> {
  return request<PreferencesResponse>(
    `/api/groups/${encodeURIComponent(groupId)}/preferences`,
    { headers: authHeaders(initData) },
  )
}

export async function savePreferences(
  initData: string,
  groupId: string,
  intervals: PreferenceInterval[],
): Promise<PreferencesResponse> {
  return request<PreferencesResponse>(
    `/api/groups/${encodeURIComponent(groupId)}/preferences`,
    {
      method: 'PUT',
      headers: {
        ...authHeaders(initData),
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ intervals }),
    },
  )
}

export async function fetchGroupCalendar(
  initData: string,
  groupId: string,
): Promise<GroupCalendarResponse> {
  return request<GroupCalendarResponse>(
    `/api/groups/${encodeURIComponent(groupId)}/calendar`,
    { headers: authHeaders(initData) },
  )
}

export async function fetchMemberCalendar(
  initData: string,
  groupId: string,
  memberId: string,
): Promise<MemberCalendarResponse> {
  return request<MemberCalendarResponse>(
    `/api/groups/${encodeURIComponent(groupId)}/members/${encodeURIComponent(memberId)}/calendar`,
    { headers: authHeaders(initData) },
  )
}

function authHeaders(initData: string): Record<string, string> {
  return { Authorization: `tma ${initData}` }
}

async function request<T>(path: string, init: RequestInit): Promise<T> {
  let response: Response
  try {
    response = await fetch(path, {
      ...init,
      headers: {
        Accept: 'application/json',
        ...(init.headers ?? {}),
      },
    })
  } catch {
    throw new ApiError('NETWORK_ERROR', 'network request failed')
  }

  const body = await readJson(response)
  if (!response.ok) {
    const envelope = body as ErrorEnvelope | null
    throw new ApiError(
      envelope?.error?.code ?? 'HTTP_ERROR',
      envelope?.error?.message ?? `request failed with ${response.status}`,
      response.status,
    )
  }

  return body as T
}

async function readJson(response: Response): Promise<unknown> {
  try {
    return await response.json()
  } catch {
    return null
  }
}
