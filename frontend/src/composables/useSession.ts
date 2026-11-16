import { computed, ref } from 'vue'

import { ApiError, authenticate, updateTimezone } from '../api/client'
import { ru } from '../i18n/ru'
import { initTelegram } from '../telegram/webapp'
import type { UserDto } from '../types/api'

export type SessionState = 'loading' | 'ready' | 'no-telegram' | 'error'

const errorMessages: Record<string, string> = { ...ru.errors }

function codeOf(error: unknown): string {
  return error instanceof ApiError ? error.code : 'UNKNOWN'
}

function messageFor(code: string | null): string {
  if (!code) return ru.errors.UNKNOWN
  return errorMessages[code] ?? ru.errors.UNKNOWN
}

export function useSession() {
  const state = ref<SessionState>('loading')
  const user = ref<UserDto | null>(null)
  const errorCode = ref<string | null>(null)
  const initData = ref('')
  const saving = ref(false)
  const saveError = ref<string | null>(null)

  const errorMessage = computed(() => messageFor(errorCode.value))
  const displayName = computed(() => {
    const current = user.value
    if (!current) return ''

    const first = current.first_name?.trim() ?? ''
    const last = current.last_name?.trim() ?? ''
    if (first && last) return `${first} ${last}`
    if (first) return first
    if (current.username) return `@${current.username}`
    return ru.common.memberFallback
  })
  const timezoneRequired = computed(
    () => state.value === 'ready' && user.value !== null && !user.value.timezone_set,
  )

  async function start(): Promise<void> {
    state.value = 'loading'
    errorCode.value = null
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
      errorCode.value = codeOf(error)
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
      saveError.value = messageFor(codeOf(error))
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
    errorCode,
    errorMessage,
    displayName,
    timezoneRequired,
    saving,
    saveError,
    start,
    saveTimezone,
    clearSaveError,
  }
}
