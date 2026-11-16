/**
 * Minimal typed wrapper around the official Telegram Mini App SDK.
 * Authentication always uses the signed `initData` string, never
 * `initDataUnsafe`.
 */
export interface TelegramWebApp {
  initData: string
  colorScheme: 'light' | 'dark'
  themeParams: Record<string, string>
  isExpanded: boolean
  ready(): void
  expand(): void
  setHeaderColor?(color: string): void
  setBackgroundColor?(color: string): void
}

declare global {
  interface Window {
    Telegram?: {
      WebApp?: TelegramWebApp
    }
  }
}

export function getWebApp(): TelegramWebApp | null {
  return window.Telegram?.WebApp ?? null
}

/** Initializes the Telegram shell and returns the WebApp instance if present. */
export function initTelegram(): TelegramWebApp | null {
  const webApp = getWebApp()
  if (!webApp) {
    return null
  }

  try {
    webApp.ready()
    webApp.expand()
    webApp.setHeaderColor?.('#0b0f19')
    webApp.setBackgroundColor?.('#0b0f19')
  } catch {
    // Older Telegram clients may not support every method.
  }

  return webApp
}

export function getInitData(): string {
  return getWebApp()?.initData ?? ''
}
