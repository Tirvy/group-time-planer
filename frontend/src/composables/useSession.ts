import { computed, ref } from 'vue'

import { authenticate, updateTimezone } from '../api/client'
import { describeError, isSessionExpired } from '../api/errors'
import { initTelegram } from '../telegram/webapp'
import type { UserDto } from '../types/api'

export type SessionState = 'loading' | 'ready' | 'no-telegram' | 'error'

export function useSession() {
  const state = ref<SessionState>('loading')
  const user = ref<UserDto | null>(null)
  const initData = ref('')
  const saving = ref(false)
  const saveError = ref<string | null>(null)

  const errorMessage = ref<string | null>(null)
  const sessionExpired = ref(false)
  const timezoneRequired = computed(
    () => state.value === 'ready' && user.value !== null && !user.value.timezone_set,
  )

  async function start(): Promise<void> {
    state.value = 'loading'
    errorMessage.value = null
    sessionExpired.value = false
    saveError.value = null

    const webApp = initTelegram()
    initData.value = webApp?.initData ?? ''
    if (!initData.value) {
      state.value = 'no-telegram'
      return
    }

    try {
      const response = await authenticate(initData.value)
      user.value = response.user
      state.value = 'ready'
    } catch (error) {
      user.value = null
      errorMessage.value = describeError(error)
      sessionExpired.value = isSessionExpired(error)
      state.value = 'error'
    }
  }

  async function saveTimezone(timezone: string): Promise<boolean> {
    saving.value = true
    saveError.value = null

    try {
      const response = await updateTimezone(initData.value, timezone)
      user.value = response.user
      return true
    } catch (error) {
      saveError.value = describeError(error)
      return false
    } finally {
      saving.value = false
    }
  }

  function clearSaveError(): void {
    saveError.value = null
  }

  return {
    state,
    user,
    initData,
    errorMessage,
    sessionExpired,
    timezoneRequired,
    saving,
    saveError,
    start,
    saveTimezone,
    clearSaveError,
  }
}
