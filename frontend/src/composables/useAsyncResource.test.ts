import { describe, expect, it, vi } from 'vitest'

import { ApiError } from '../api/client'
import { ru } from '../i18n/ru'
import { useAsyncResource } from './useAsyncResource'

describe('useAsyncResource', () => {
  it('loads once per key and reuses cached data', async () => {
    const loader = vi.fn(async (key: string) => `value:${key}`)
    const resource = useAsyncResource(loader, 'fallback')

    await resource.run('a')
    await resource.run('a')

    expect(loader).toHaveBeenCalledTimes(1)
    expect(resource.data.value).toBe('value:a')
    expect(resource.loading.value).toBe(false)

    await resource.run('b')
    expect(loader).toHaveBeenCalledTimes(2)
    expect(resource.data.value).toBe('value:b')
  })

  it('ignores a response from a request invalidated by reset', async () => {
    let resolveFirst: (value: string) => void = () => {}
    const loader = vi.fn(
      () =>
        new Promise<string>((resolve) => {
          resolveFirst = resolve
        }),
    )
    const resource = useAsyncResource(loader, 'fallback')

    const pending = resource.run('group-1')
    expect(resource.loading.value).toBe(true)

    resource.reset()
    resolveFirst('stale')
    await pending

    expect(resource.data.value).toBeNull()
    expect(resource.loading.value).toBe(false)
  })

  it('lets a newer key supersede an in-flight request', async () => {
    const resolvers = new Map<string, (value: string) => void>()
    const loader = vi.fn(
      (key: string) =>
        new Promise<string>((resolve) => {
          resolvers.set(key, resolve)
        }),
    )
    const resource = useAsyncResource(loader, 'fallback')

    const first = resource.run('member-1')
    const second = resource.run('member-2')

    resolvers.get('member-2')?.('member-2 data')
    await second
    expect(resource.data.value).toBe('member-2 data')

    // The stale response must not overwrite the newer selection.
    resolvers.get('member-1')?.('member-1 data')
    await first
    expect(resource.data.value).toBe('member-2 data')
    expect(resource.loading.value).toBe(false)
  })

  it('maps API error codes to centralized copy', async () => {
    const loader = vi.fn(async () => {
      throw new ApiError('AUTH_EXPIRED', 'expired', 401)
    })
    const resource = useAsyncResource(loader, 'fallback')

    await resource.run('a')

    expect(resource.error.value).toBe(ru.errors.AUTH_EXPIRED)
    expect(resource.data.value).toBeNull()
  })

  it('falls back for unknown failures and recovers on retry', async () => {
    let shouldFail = true
    const loader = vi.fn(async () => {
      if (shouldFail) throw new Error('boom')
      return 'ok'
    })
    const resource = useAsyncResource(loader, 'calendar failed')

    await resource.run('a')
    expect(resource.error.value).toBe('calendar failed')

    shouldFail = false
    await resource.retry('a')

    expect(resource.error.value).toBeNull()
    expect(resource.data.value).toBe('ok')
  })
})
