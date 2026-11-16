<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { ru } from '../i18n/ru'
import { deviceTimezone, formatOffset, listTimezones } from '../timezones'

const props = defineProps<{
  current: string | null
  saving: boolean
  error: string | null
}>()

const emit = defineEmits<{
  save: [timezone: string]
  cancel: []
}>()

const options = ref<string[]>(listTimezones())
const suggested = deviceTimezone()
const selected = ref<string | null>(props.current ?? suggested ?? '')
const confirmOpen = ref(false)

watch(
  () => props.current,
  (value) => {
    if (value) {
      selected.value = value
    }
  },
)

const isChange = computed(() => props.current !== null)
const selectedOffset = computed(() =>
  selected.value ? formatOffset(selected.value) : '',
)
const currentOffset = computed(() =>
  props.current ? formatOffset(props.current) : '',
)
const canSave = computed(
  () =>
    selected.value !== null &&
    selected.value.length > 0 &&
    !props.saving &&
    selected.value !== props.current,
)
const suggestionAvailable = computed(
  () =>
    suggested !== null &&
    suggested !== selected.value &&
    options.value.includes(suggested),
)

function useSuggestion(): void {
  if (suggested) {
    selected.value = suggested
  }
}

function requestSave(): void {
  if (!canSave.value || !selected.value) return
  if (isChange.value && selected.value !== props.current) {
    confirmOpen.value = true
    return
  }
  emit('save', selected.value)
}

function confirmChange(): void {
  confirmOpen.value = false
  if (selected.value) {
    emit('save', selected.value)
  }
}
</script>

<template>
  <v-card class="timezone-card" variant="flat">
    <v-card-item>
      <v-card-title class="text-h6">
        {{ isChange ? ru.timezone.changeTitle : ru.timezone.title }}
      </v-card-title>
      <v-card-subtitle class="text-wrap">
        {{ isChange ? ru.timezone.changeWarning : ru.timezone.body }}
      </v-card-subtitle>
    </v-card-item>

    <v-card-text>
      <v-alert
        v-if="props.error"
        type="error"
        variant="tonal"
        density="comfortable"
        class="mb-4"
        role="alert"
      >
        {{ props.error }}
      </v-alert>

      <template v-if="isChange">
        <div class="text-caption text-medium-emphasis mb-1">
          {{ ru.timezone.current }}
        </div>
        <div class="text-body-1 mb-4">
          {{ props.current }}
          <span v-if="currentOffset" class="text-medium-emphasis">
            · {{ currentOffset }}
          </span>
        </div>
      </template>

      <v-autocomplete
        v-model="selected"
        :items="options"
        :label="ru.timezone.label"
        :hint="ru.timezone.searchHint"
        persistent-hint
        clearable
        density="comfortable"
        variant="outlined"
        :no-data-text="ru.timezone.noResults"
        autocomplete="off"
      />

      <div v-if="selected" class="d-flex align-center ga-2 mt-4">
        <v-chip size="small" color="primary" variant="tonal">
          {{ selected }}
        </v-chip>
        <span v-if="selectedOffset" class="text-caption text-medium-emphasis">
          {{ selectedOffset }}
        </span>
      </div>

      <div v-if="suggested" class="mt-4">
        <div class="text-caption text-medium-emphasis">
          {{ ru.timezone.suggested }}: {{ suggested }}
        </div>
        <v-btn
          v-if="suggestionAvailable"
          class="mt-2"
          size="small"
          variant="text"
          color="primary"
          @click="useSuggestion"
        >
          {{ ru.timezone.useSuggested }}
        </v-btn>
      </div>

      <v-alert
        v-if="!isChange"
        type="info"
        variant="tonal"
        density="compact"
        class="mt-4"
      >
        {{ ru.timezone.requiredHint }}
      </v-alert>
    </v-card-text>

    <v-card-actions class="px-4 pb-4">
      <v-spacer />
      <v-btn v-if="isChange" variant="text" :disabled="props.saving" @click="emit('cancel')">
        {{ ru.common.cancel }}
      </v-btn>
      <v-btn
        color="primary"
        variant="flat"
        :loading="props.saving"
        :disabled="!canSave"
        @click="requestSave"
      >
        {{ props.saving ? ru.timezone.saving : ru.timezone.save }}
      </v-btn>
    </v-card-actions>
  </v-card>

  <v-dialog v-model="confirmOpen" max-width="420">
    <v-card>
      <v-card-title>{{ ru.timezone.confirmTitle }}</v-card-title>
      <v-card-text>{{ ru.timezone.confirmBody }}</v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="confirmOpen = false">
          {{ ru.common.cancel }}
        </v-btn>
        <v-btn color="primary" variant="flat" @click="confirmChange">
          {{ ru.timezone.confirm }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.timezone-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
}
</style>
