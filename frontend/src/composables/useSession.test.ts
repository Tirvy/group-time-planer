import { beforeEach, describe, expect, it, vi } from 'vitest'

import { useSession } from './useSession'
import type { UserDto } from '../types/api'
import { ru } from '../i18n/ru'

vi.mock('../api/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api/client')>()
  return { ...actual, authenticate: vi.fn(), updateTimezone: vi.fn() }
})

vi.mock('../telegram/webapp', () => ({
  initTelegram: vi.fn(),
}))

const { ApiError, authenticate, updateTimezone } = await import('../api/client')
const { initTelegram } = await import('../telegram/webapp')

const user: UserDto = {
  telegram_id: '42',
  username: 'tester',
  first_name: 'Иван',
  last_name: null,
  timezone: 'Europe/Berlin',
  timezone_set: true,
}

describe('useSession', () => {
  beforeEach(() => {
    vi.mocked(authenticate).mockReset()
    vi.mocked(updateTimezone).mockReset()
    vi.mocked(initTelegram).mockReset()
  })

  it('reports a missing Telegram shell', async () => {
    vi.mocked(initTelegram).mockReturnValue(null)
    const session = useSession()

    await session.start()

    expect(session.state.value).toBe('no-telegram')
    expect(authenticate).not.toHaveBeenCalled()
  })

  it('authenticates with initData and stores the profile', async () => {
    vi.mocked(initTelegram).mockReturnValue({
      initData: 'signed',
      ready: vi.fn(),
      expand: vi.fn(),
      isExpanded: true,
    })
    vi.mocked(authenticate).mockResolvedValue({ user })
    const session = useSession()

    await session.start()

    expect(authenticate).toHaveBeenCalledWith('signed')
    expect(session.state.value).toBe('ready')
    expect(session.user.value).toEqual(user)
    expect(session.timezoneRequired.value).toBe(false)
  })

  it('flags an expired session and maps the error copy', async () => {
    vi.mocked(initTelegram).mockReturnValue({
      initData: 'signed',
      ready: vi.fn(),
      expand: vi.fn(),
      isExpanded: true,
    })
    vi.mocked(authenticate).mockRejectedValue(
      new ApiError('AUTH_EXPIRED', 'expired', 401),
    )
    const session = useSession()

    await session.start()

    expect(session.state.value).toBe('error')
    expect(session.errorMessage.value).toBe(ru.errors.AUTH_EXPIRED)
    expect(session.sessionExpired.value).toBe(true)
  })

  it('does not show the reopen hint for transient failures', async () => {
    vi.mocked(initTelegram).mockReturnValue({
      initData: 'signed',
      ready: vi.fn(),
      expand: vi.fn(),
      isExpanded: true,
    })
    vi.mocked(authenticate).mockRejectedValue(
      new ApiError('NETWORK_ERROR', 'offline', null),
    )
    const session = useSession()

    await session.start()

    expect(session.errorMessage.value).toBe(ru.errors.NETWORK_ERROR)
    expect(session.sessionExpired.value).toBe(false)
  })

  it('saves the timezone and surfaces failures without leaving the editor', async () => {
    vi.mocked(initTelegram).mockReturnValue({
      initData: 'signed',
      ready: vi.fn(),
      expand: vi.fn(),
      isExpanded: true,
    })
    vi.mocked(authenticate).mockResolvedValue({ user })
    vi.mocked(updateTimezone).mockResolvedValue({
      user: { ...user, timezone: 'Asia/Tbilisi' },
    })
    const session = useSession()
    await session.start()

    const saved = await session.saveTimezone('Asia/Tbilisi')

    expect(saved).toBe(true)
    expect(session.user.value?.timezone).toBe('Asia/Tbilisi')
    expect(session.saveError.value).toBeNull()

    vi.mocked(updateTimezone).mockRejectedValue(
      new ApiError('TIMEZONE_INVALID', 'bad zone', 400),
    )
    const failed = await session.saveTimezone('Mars/Olympus')

    expect(failed).toBe(false)
    expect(session.saveError.value).toBe(ru.errors.TIMEZONE_INVALID)
    expect(session.user.value?.timezone).toBe('Asia/Tbilisi')
  })
})
