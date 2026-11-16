import '@mdi/font/css/materialdesignicons.css'
import 'vuetify/styles'

import { createVuetify } from 'vuetify'
import { ru } from 'vuetify/locale'

// Mini App palette: dark, compact and readable on small Telegram viewports.
export const vuetify = createVuetify({
  locale: {
    locale: 'ru',
    fallback: 'en',
    messages: { ru },
  },
  theme: {
    defaultTheme: 'dark',
    themes: {
      dark: {
        colors: {
          background: '#0b0f19',
          surface: '#141b2b',
          primary: '#4dd0e1',
          secondary: '#7e57c2',
          success: '#66bb6a',
          error: '#ef5350',
        },
      },
    },
  },
  defaults: {
    VBtn: { rounded: 'lg' },
    VCard: { rounded: 'xl' },
  },
})
