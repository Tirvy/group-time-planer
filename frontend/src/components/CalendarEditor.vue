<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

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
  <v-card variant="tonal" class="calendar-card">
    <v-card-item>
      <div class="d-flex align-center flex-wrap ga-2">
        <div>
          <v-card-title class="text-h6">
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
          prepend-icon="mdi-circle-edit-outline"
        >
          {{ ru.planner.unsaved }}
        </v-chip>
        <v-chip
          v-else-if="!pending"
          size="small"
          color="success"
          variant="tonal"
          prepend-icon="mdi-check-circle-outline"
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

      <div class="d-flex align-center flex-wrap ga-2 mb-3">
        <v-btn
          size="small"
          variant="text"
          prepend-icon="mdi-eraser"
          :disabled="selectedSlots.length === 0 || props.saving"
          @click="clearAll"
        >
          {{ ru.planner.clear }}
        </v-btn>
        <v-btn
          size="small"
          variant="text"
          prepend-icon="mdi-undo"
          :disabled="!dirty || props.saving"
          @click="discard"
        >
          {{ ru.planner.discard }}
        </v-btn>
        <v-spacer />
        <v-btn
          size="small"
          color="primary"
          variant="flat"
          prepend-icon="mdi-content-save-outline"
          :loading="props.saving"
          :disabled="!canSave"
          @click="requestSave"
        >
          {{ props.saving ? ru.planner.saving : ru.planner.save }}
        </v-btn>
      </div>

      <WeekGrid v-model="selectedSlots" :color="props.group.color" />

      <div class="d-flex flex-wrap align-center ga-3 mt-3">
        <span class="d-inline-flex align-center ga-2 text-caption">
          <span
            class="legend-swatch"
            :style="{ backgroundColor: props.group.color }"
            aria-hidden="true"
          />
          {{ ru.planner.legendSelected }}
        </span>
        <span class="d-inline-flex align-center ga-2 text-caption text-medium-emphasis">
          <span class="legend-swatch legend-swatch--blank" aria-hidden="true" />
          {{ ru.planner.legendBlank }}
        </span>
      </div>

      <p class="text-caption text-medium-emphasis mt-2 mb-0">
        {{ ru.planner.hint }}
      </p>

      <div class="d-flex align-center ga-2 mt-4">
        <v-spacer />
        <v-btn
          variant="text"
          :disabled="!dirty || props.saving"
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
    </v-card-text>
  </v-card>
</template>

<style scoped>
.calendar-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.legend-swatch {
  width: 14px;
  height: 14px;
  border-radius: 4px;
  display: inline-block;
}

.legend-swatch--blank {
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.2);
}
</style>
