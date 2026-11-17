import 'vuetify/styles'

import { mdi } from 'vuetify/iconsets/mdi-svg'
import { createVuetify } from 'vuetify'
import { ru } from 'vuetify/locale'

// Mini App palette: compact and readable on small Telegram viewports. The
// active scheme is chosen from Telegram's `colorScheme` at startup; these
// fallbacks are used outside Telegram and before the theme is applied.
export const vuetify = createVuetify({
  locale: {
    locale: 'ru',
    fallback: 'en',
    messages: { ru },
  },
  icons: {
    defaultSet: 'mdi',
    sets: { mdi },
  },
  theme: {
    defaultTheme: 'dark',
    themes: {
      dark: {
        dark: true,
        colors: {
          background: '#0b0f19',
          surface: '#141b2b',
          'on-background': '#e8eefc',
          'on-surface': '#e8eefc',
          primary: '#4dd0e1',
          'on-primary': '#06222a',
          secondary: '#7e57c2',
          'on-secondary': '#ffffff',
          success: '#66bb6a',
          error: '#ef5350',
        },
      },
      light: {
        dark: false,
        colors: {
          background: '#f2f4f8',
          surface: '#ffffff',
          'on-background': '#16181d',
          'on-surface': '#16181d',
          primary: '#1565c0',
          'on-primary': '#ffffff',
          secondary: '#6a4bb5',
          'on-secondary': '#ffffff',
          success: '#2e7d32',
          error: '#c62828',
        },
      },
    },
  },
  defaults: {
    VBtn: { rounded: 'lg' },
    VCard: { rounded: 'xl' },
    VAutocomplete: { rounded: 'lg' },
  },
})
