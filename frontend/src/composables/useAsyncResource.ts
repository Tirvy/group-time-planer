import { ref, type Ref } from 'vue'

import { describeError } from '../api/errors'

export interface AsyncResource<T> {
  data: Ref<T | null>
  loading: Ref<boolean>
  error: Ref<string | null>
  /** Loads once per key unless `force` is set. Concurrent calls are ignored. */
  run: (key: string, force?: boolean) => Promise<T | null>
  /** Loads again, discarding the cached value. Used by retry buttons. */
  retry: (key: string) => Promise<T | null>
  /** Drops data and invalidates any in-flight request. */
  reset: () => void
}

/**
 * Request-guarded loader for one server resource keyed by group/member identity.
 *
 * Every Mini App screen used to repeat the same "request id + loading + error +
 * stale response" bookkeeping by hand. Keeping it in one place makes the
 * behaviour consistent and testable, and makes it impossible for a response
 * from a previously selected group to overwrite the current one.
 */
export function useAsyncResource<T>(
  loader: (key: string) => Promise<T>,
  fallbackMessage: string,
): AsyncResource<T> {
  const data = ref<T | null>(null) as Ref<T | null>
  const loading = ref(false)
  const error = ref<string | null>(null)

  let requestId = 0
  let currentKey: string | null = null

  async function run(key: string, force = false): Promise<T | null> {
    if (!force && currentKey === key && data.value !== null) {
      return data.value
    }
    // Already loading this exact key: nothing to do. A different key is allowed
    // to supersede the in-flight request; the request-id guard below discards
    // whichever response arrives out of order.
    if (loading.value && currentKey === key) {
      return null
    }

    const id = ++requestId
    currentKey = key
    loading.value = true
    error.value = null

    try {
      const result = await loader(key)
      if (id !== requestId) return null
      data.value = result
      return result
    } catch (caught) {
      if (id === requestId) {
        error.value = describeError(caught, fallbackMessage)
      }
      return null
    } finally {
      if (id === requestId) {
        loading.value = false
      }
    }
  }

  function retry(key: string): Promise<T | null> {
    data.value = null
    return run(key, true)
  }

  function reset(): void {
    requestId += 1
    currentKey = null
    data.value = null
    loading.value = false
    error.value = null
  }

  return { data, loading, error, run, retry, reset }
}
