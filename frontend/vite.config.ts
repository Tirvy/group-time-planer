import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import { defineConfig } from 'vitest/config'

// The backend serves the built assets in production. During development the
// Mini App shell runs on Vite and proxies API traffic to the Rust process.
export default defineConfig({
  plugins: [vue(), vuetify({ autoImport: true })],
  server: {
    port: 5173,
    proxy: {
      '/api': { target: 'http://127.0.0.1:8080', changeOrigin: true },
      '/health': { target: 'http://127.0.0.1:8080', changeOrigin: true },
    },
  },
  test: {
    environment: 'happy-dom',
    include: ['src/**/*.test.ts'],
    // Vuetify components import their own CSS; inline the package so Vitest
    // processes those imports instead of handing them to Node.
    server: {
      deps: {
        inline: ['vuetify'],
      },
    },
  },
})
