import { ru } from '../i18n/ru'
import { ApiError } from './client'

const errorCopy: Record<string, string> = ru.errors

/**
 * Maps a thrown error to centralized Russian copy.
 *
 * All API failures carry a stable machine-readable `code`, so components never
 * show `error.message` directly and never hardcode their own copy tables.
 */
export function describeError(error: unknown, fallback: string = ru.errors.UNKNOWN): string {
  if (error instanceof ApiError) {
    return errorCopy[error.code] ?? fallback
  }
  return fallback
}

/** True when the Telegram session must be refreshed by reopening the Mini App. */
export function isSessionExpired(error: unknown): boolean {
  return (
    error instanceof ApiError &&
    (error.code === 'AUTH_EXPIRED' || error.code === 'AUTH_INVALID' || error.code === 'AUTH_MISSING')
  )
}
