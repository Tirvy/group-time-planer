import { describe, expect, it, vi } from 'vitest'

import { vuetify } from '../plugins/vuetify'
import { applyTelegramTheme, resolveTelegramTheme } from './theme'

describe('resolveTelegramTheme', () => {
  it('returns null outside Telegram', () => {
    expect(resolveTelegramTheme(null)).toBeNull()
    expect(resolveTelegramTheme(undefined)).toBeNull()
  })

  it('uses the dark fallback palette when Telegram reports dark', () => {
    const theme = resolveTelegramTheme({ colorScheme: 'dark' })

    expect(theme).not.toBeNull()
    expect(theme?.name).toBe('dark')
    expect(theme?.background).toBe('#0b0f19')
    expect(theme?.surface).toBe('#141b2b')
    expect(theme?.primary).toBe('#4dd0e1')
  })

  it('uses the light fallback palette when Telegram reports light', () => {
    const theme = resolveTelegramTheme({ colorScheme: 'light' })

    expect(theme?.name).toBe('light')
    expect(theme?.background).toBe('#f2f4f8')
    expect(theme?.surface).toBe('#ffffff')
    expect(theme?.text).toBe('#16181d')
  })

  it('prefers Telegram palette values over the fallbacks', () => {
    const theme = resolveTelegramTheme({
      colorScheme: 'light',
      themeParams: {
        bg_color: '#fff8e1',
        secondary_bg_color: '#ffffff',
        text_color: '#1a1a1a',
        button_color: '#0f7b6c',
        button_text_color: '#ffffff',
        header_bg_color: '#fff3cd',
      },
    })

    expect(theme).toEqual({
      name: 'light',
      background: '#fff8e1',
      surface: '#ffffff',
      primary: '#0f7b6c',
      onPrimary: '#ffffff',
      text: '#1a1a1a',
      header: '#fff3cd',
    })
  })

  it('treats a missing color scheme as dark', () => {
    expect(resolveTelegramTheme({ themeParams: {} })?.name).toBe('dark')
  })

  it('applies the resolved scheme to Vuetify and the Telegram chrome', () => {
    const setHeaderColor = vi.fn()
    const setBackgroundColor = vi.fn()

    applyTelegramTheme({
      initData: '',
      isExpanded: true,
      ready: vi.fn(),
      expand: vi.fn(),
      colorScheme: 'light',
      themeParams: { bg_color: '#fff8e1', text_color: '#101010' },
      setHeaderColor,
      setBackgroundColor,
    })

    expect(vuetify.theme.global.name.value).toBe('light')
    expect(vuetify.theme.themes.value.light?.colors.background).toBe('#fff8e1')
    expect(vuetify.theme.themes.value.light?.colors['on-background']).toBe('#101010')
    expect(setHeaderColor).toHaveBeenCalledWith('#fff8e1')
    expect(setBackgroundColor).toHaveBeenCalledWith('#fff8e1')

    // Restore the default theme so other tests are not affected.
    vuetify.theme.global.name.value = 'dark'
  })
})
