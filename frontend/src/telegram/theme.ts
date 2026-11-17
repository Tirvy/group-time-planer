import { vuetify } from '../plugins/vuetify'
import { getWebApp, type TelegramWebApp } from './webapp'

export interface TelegramThemeSource {
  colorScheme?: 'light' | 'dark'
  themeParams?: Record<string, string>
}

export interface ResolvedTheme {
  name: 'light' | 'dark'
  background: string
  surface: string
  primary: string
  onPrimary: string
  text: string
  header: string
}

interface ThemeFallback {
  background: string
  surface: string
  primary: string
  onPrimary: string
  text: string
  header: string
}

const DARK_FALLBACK: ThemeFallback = {
  background: '#0b0f19',
  surface: '#141b2b',
  primary: '#4dd0e1',
  onPrimary: '#06222a',
  text: '#e8eefc',
  header: '#0b0f19',
}

const LIGHT_FALLBACK: ThemeFallback = {
  background: '#f2f4f8',
  surface: '#ffffff',
  primary: '#1565c0',
  onPrimary: '#ffffff',
  text: '#16181d',
  header: '#ffffff',
}

/**
 * Maps Telegram's theme report onto Vuetify theme colors.
 *
 * Telegram exposes a scheme (`light`/`dark`) plus optional palette values. When
 * a value is missing we fall back to the app's own palette, so the Mini App
 * always renders with a readable, self-consistent theme.
 */
export function resolveTelegramTheme(
  source: TelegramThemeSource | null | undefined,
): ResolvedTheme | null {
  if (!source) {
    return null
  }

  const name = source.colorScheme === 'light' ? 'light' : 'dark'
  const params = source.themeParams ?? {}
  const fallback = name === 'light' ? LIGHT_FALLBACK : DARK_FALLBACK

  return {
    name,
    background: params.bg_color ?? fallback.background,
    surface: params.secondary_bg_color ?? fallback.surface,
    primary: params.button_color ?? fallback.primary,
    onPrimary: params.button_text_color ?? fallback.onPrimary,
    text: params.text_color ?? fallback.text,
    header: params.header_bg_color ?? params.bg_color ?? fallback.header,
  }
}

/**
 * Applies the Telegram scheme to Vuetify and the Telegram chrome.
 *
 * Called once at startup and again on `themeChanged`, so switching Telegram's
 * appearance updates the Mini App without a reload.
 */
export function applyTelegramTheme(
  webApp: TelegramWebApp | null = getWebApp(),
): ResolvedTheme | null {
  const resolved = resolveTelegramTheme(webApp)
  if (!resolved || !webApp) {
    return null
  }

  vuetify.theme.global.name.value = resolved.name

  const theme = vuetify.theme.themes.value[resolved.name]
  const colors = theme?.colors as unknown as Record<string, string> | undefined
  if (colors) {
    colors.background = resolved.background
    colors.surface = resolved.surface
    colors.primary = resolved.primary
    colors['on-primary'] = resolved.onPrimary
    colors['on-background'] = resolved.text
    colors['on-surface'] = resolved.text
  }

  document.documentElement.dataset.theme = resolved.name
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute('content', resolved.background)

  try {
    webApp.setHeaderColor?.(resolved.header)
    webApp.setBackgroundColor?.(resolved.background)
  } catch {
    // Older Telegram clients may not support the color methods.
  }

  return resolved
}

/** Applies the current theme and keeps it in sync with Telegram. */
export function watchTelegramTheme(
  webApp: TelegramWebApp | null = getWebApp(),
): ResolvedTheme | null {
  const resolved = applyTelegramTheme(webApp)
  try {
    webApp?.onEvent?.('themeChanged', () => applyTelegramTheme(webApp))
  } catch {
    // Ignore clients without theme events.
  }
  return resolved
}
