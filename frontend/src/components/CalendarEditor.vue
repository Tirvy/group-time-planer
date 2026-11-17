<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { mdiCheckCircleOutline, mdiCircleEditOutline } from '@mdi/js'

import { ru } from '../i18n/ru'
import { intervalsToSlots, sameSlots, slotsToIntervals } from '../planner/model'
import type { GroupDto, PreferenceInterval, PreferencesResponse } from '../types/api'
import WeekGrid from './WeekGrid.vue'

const props = defineProps<{
  group: GroupDto
  preferences: PreferencesResponse
  saving: boolean
  saveError: string | null
}>()

const emit = defineEmits<{
  save: [intervals: PreferenceInterval[]]
  dirty: [value: boolean]
}>()

const savedSlots = ref<number[]>(intervalsToSlots(props.preferences.intervals))
const selectedSlots = ref<number[]>([...savedSlots.value])

const dirty = computed(() => !sameSlots(selectedSlots.value, savedSlots.value))
const pending = computed(() => !props.preferences.setup_completed)
const completedEmpty = computed(
  () => props.preferences.setup_completed && savedSlots.value.length === 0,
)
const canSave = computed(
  () => (dirty.value || pending.value) && !props.saving,
)

watch(
  dirty,
  (value) => emit('dirty', value),
  { immediate: true },
)

watch(
  () => props.preferences,
  (preferences) => {
    const slots = intervalsToSlots(preferences.intervals)
    savedSlots.value = slots
    selectedSlots.value = [...slots]
  },
  { deep: true },
)

function requestSave(): void {
  if (!canSave.value) return
  emit('save', slotsToIntervals(selectedSlots.value))
}

function discard(): void {
  if (!dirty.value) return
  selectedSlots.value = [...savedSlots.value]
}

function clearAll(): void {
  if (selectedSlots.value.length === 0) return
  selectedSlots.value = []
}

function handleBeforeUnload(event: BeforeUnloadEvent): void {
  if (!dirty.value) return
  event.preventDefault()
  event.returnValue = ''
}

onMounted(() => window.addEventListener('beforeunload', handleBeforeUnload))
onBeforeUnmount(() => window.removeEventListener('beforeunload', handleBeforeUnload))
</script>

<template>
  <v-card
    variant="tonal"
    class="calendar-card"
    :class="{ 'calendar-card--actions': dirty }"
  >
    <v-card-item>
      <div class="d-flex align-center flex-wrap ga-2">
        <div>
          <v-card-title class="text-title-large">
            {{ ru.planner.calendarTitle }}
          </v-card-title>
          <v-card-subtitle class="text-wrap">
            {{ ru.planner.calendarSubtitle }}
          </v-card-subtitle>
        </div>
        <v-spacer />
        <v-chip
          v-if="dirty"
          size="small"
          color="warning"
          variant="tonal"
          :prepend-icon="mdiCircleEditOutline"
        >
          {{ ru.planner.unsaved }}
        </v-chip>
        <v-chip
          v-else-if="!pending"
          size="small"
          color="success"
          variant="tonal"
          :prepend-icon="mdiCheckCircleOutline"
        >
          {{ ru.planner.configuredBanner }}
        </v-chip>
      </div>
    </v-card-item>

    <v-card-text class="pt-0">
      <v-alert
        v-if="pending"
        type="info"
        variant="tonal"
        density="comfortable"
        class="mb-3"
      >
        {{ ru.planner.pendingBanner }}
      </v-alert>
      <v-alert
        v-else-if="completedEmpty"
        type="info"
        variant="tonal"
        density="comfortable"
        class="mb-3"
      >
        {{ ru.planner.emptyCompletedBanner }}
      </v-alert>

      <v-alert
        v-if="props.saveError"
        type="error"
        variant="tonal"
        density="comfortable"
        class="mb-3"
        role="alert"
      >
        {{ props.saveError }}
      </v-alert>

      <div class="d-flex flex-wrap align-center ga-3 mb-2">
        <span class="d-inline-flex align-center ga-2 text-body-small">
          <span
            class="legend-swatch"
            :style="{ backgroundColor: props.group.color }"
            aria-hidden="true"
          />
          {{ ru.planner.legendSelected }}
        </span>
        <span class="d-inline-flex align-center ga-2 text-body-small text-medium-emphasis">
          <span class="legend-swatch legend-swatch--blank" aria-hidden="true" />
          {{ ru.planner.legendBlank }}
        </span>
      </div>

      <WeekGrid v-model="selectedSlots" :color="props.group.color" />

      <p class="text-body-small text-medium-emphasis mt-2 mb-0">
        {{ ru.planner.hint }}
      </p>
    </v-card-text>
  </v-card>

  <v-slide-y-transition>
    <div
      v-if="dirty"
      class="action-bar"
      role="toolbar"
      :aria-label="ru.planner.unsaved"
    >
      <div class="action-bar__inner">
        <v-btn
          variant="text"
          :disabled="selectedSlots.length === 0 || props.saving"
          @click="clearAll"
        >
          {{ ru.planner.clear }}
        </v-btn>
        <v-spacer />
        <v-btn
          variant="text"
          :disabled="props.saving"
          @click="discard"
        >
          {{ ru.planner.discard }}
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :loading="props.saving"
          :disabled="!canSave"
          @click="requestSave"
        >
          {{ props.saving ? ru.planner.saving : ru.planner.save }}
        </v-btn>
      </div>
    </div>
  </v-slide-y-transition>
</template>

<style scoped>
.calendar-card {
  border: 1px solid var(--app-border);
}

.calendar-card--actions {
  margin-bottom: calc(84px + env(safe-area-inset-bottom, 0px));
}

.action-bar {
  position: fixed;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 1000;
  padding: 8px 16px calc(8px + env(safe-area-inset-bottom, 0px));
  border-top: 1px solid var(--app-border);
  background: rgba(var(--v-theme-surface), 0.96);
  box-shadow: 0 -8px 24px var(--app-shadow);
  backdrop-filter: blur(10px);
}

.action-bar__inner {
  display: flex;
  align-items: center;
  gap: 8px;
  max-width: 560px;
  margin: 0 auto;
}

.legend-swatch {
  width: 14px;
  height: 14px;
  border-radius: 4px;
  display: inline-block;
}

.legend-swatch--blank {
  background: var(--app-cell-bg);
  border: 1px solid var(--app-cell-border);
}
</style>
