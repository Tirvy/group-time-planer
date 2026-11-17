<script setup lang="ts">
import { mdiRefresh } from '@mdi/js'

import { ru } from '../i18n/ru'

defineProps<{
  loading: boolean
  error: string | null
  loadingText: string
}>()

defineEmits<{
  retry: []
}>()
</script>

<template>
  <div
    v-if="loading"
    class="d-flex flex-column align-center py-8"
    role="status"
    aria-live="polite"
  >
    <v-progress-circular indeterminate color="primary" size="28" />
    <p class="text-body-medium text-medium-emphasis mt-3 mb-0">
      {{ loadingText }}
    </p>
  </div>

  <v-card v-else-if="error" variant="tonal" class="state-card">
    <v-card-text>
      <v-alert type="error" variant="tonal" density="comfortable" role="alert">
        {{ error }}
      </v-alert>
    </v-card-text>
    <v-card-actions class="px-4 pb-4">
      <slot name="error-actions" />
      <v-spacer />
      <v-btn
        color="primary"
        variant="flat"
        :prepend-icon="mdiRefresh"
        @click="$emit('retry')"
      >
        {{ ru.planner.retry }}
      </v-btn>
    </v-card-actions>
  </v-card>

  <slot v-else />
</template>

<style scoped>
.state-card {
  border: 1px solid var(--app-border);
}
</style>
