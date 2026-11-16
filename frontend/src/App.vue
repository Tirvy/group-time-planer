<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'

import { ApiError, fetchHealth } from './api/client'
import { ru } from './i18n/ru'
import type { HealthResponse } from './types/api'

type LoadState = 'loading' | 'ready' | 'degraded' | 'error'

const state = ref<LoadState>('loading')
const health = ref<HealthResponse | null>(null)
const errorMessage = ref('')

const errorMessages: Record<string, string> = { ...ru.errors }

const alertType = computed(() => {
  if (state.value === 'ready') return 'success'
  if (state.value === 'degraded') return 'warning'
  return 'error'
})

const alertTitle = computed(() => {
  if (state.value === 'ready') return ru.status.okTitle
  if (state.value === 'degraded') return ru.status.degradedTitle
  return ru.status.errorTitle
})

const alertBody = computed(() => {
  if (state.value === 'ready') return ru.status.okBody
  if (state.value === 'degraded') return ru.status.degradedBody
  return errorMessage.value || ru.status.errorBody
})

async function check(): Promise<void> {
  state.value = 'loading'
  errorMessage.value = ''

  try {
    const result = await fetchHealth()
    health.value = result
    state.value = result.status === 'ok' ? 'ready' : 'degraded'
  } catch (error) {
    health.value = null
    errorMessage.value =
      error instanceof ApiError
        ? (errorMessages[error.code] ?? ru.errors.UNKNOWN)
        : ru.errors.UNKNOWN
    state.value = 'error'
  }
}

onMounted(check)
</script>

<template>
  <v-app>
    <v-app-bar flat color="background" density="comfortable">
      <v-app-bar-title>
        <div class="d-flex flex-column">
          <span class="text-subtitle-1 font-weight-bold">{{ ru.app.title }}</span>
          <span class="text-caption text-medium-emphasis">{{ ru.app.subtitle }}</span>
        </div>
      </v-app-bar-title>
    </v-app-bar>

    <v-main>
      <v-container class="app-shell pt-4" max-width="560">
        <v-card variant="tonal" class="status-card">
          <v-card-text>
            <div
              v-if="state === 'loading'"
              class="d-flex align-center ga-3 py-2"
              role="status"
              aria-live="polite"
            >
              <v-progress-circular indeterminate size="22" width="2" color="primary" />
              <span class="text-body-2">{{ ru.status.checking }}</span>
            </div>

            <template v-else>
              <v-alert
                :type="alertType"
                variant="tonal"
                density="comfortable"
                class="mb-4"
                role="status"
                aria-live="polite"
              >
                <div class="font-weight-bold">{{ alertTitle }}</div>
                <div class="text-body-2">{{ alertBody }}</div>
              </v-alert>

              <div v-if="health" class="d-flex flex-wrap ga-2">
                <v-chip
                  size="small"
                  variant="tonal"
                  :color="health.database === 'ok' ? 'success' : 'error'"
                  :prepend-icon="
                    health.database === 'ok' ? 'mdi-database-check' : 'mdi-database-alert'
                  "
                >
                  {{
                    health.database === 'ok'
                      ? ru.status.databaseOk
                      : ru.status.databaseError
                  }}
                </v-chip>
                <v-chip size="small" variant="outlined">
                  {{ ru.status.version }} {{ health.version }}
                </v-chip>
              </div>

              <v-btn
                v-if="state === 'error' || state === 'degraded'"
                class="mt-4"
                color="primary"
                variant="flat"
                prepend-icon="mdi-refresh"
                @click="check"
              >
                {{ ru.status.retry }}
              </v-btn>
            </template>
          </v-card-text>
        </v-card>

        <p class="text-caption text-medium-emphasis mt-6 mb-0">
          {{ ru.app.stageNote }}
        </p>
      </v-container>
    </v-main>
  </v-app>
</template>

<style scoped>
.app-shell {
  padding-bottom: calc(24px + env(safe-area-inset-bottom, 0px));
}

.status-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
}
</style>
