<script setup lang="ts">
import { computed, ref } from 'vue'

import { ru } from '../i18n/ru'
import { buildCombinedGrid } from '../planner/combined'
import {
  addDaysIso,
  formatDateShort,
  formatWeekRange,
} from '../planner/dates'
import { DAYS, formatTime } from '../planner/model'
import {
  memberDisplayName,
  memberRoleLabel,
  memberSetupLabel,
} from '../planner/names'
import type { CalendarMember, GroupCalendarResponse } from '../types/api'

const READ_SLOT_MINUTES = 15
const READ_SLOTS_PER_DAY = 96

const props = defineProps<{
  calendar: GroupCalendarResponse
  dirty: boolean
}>()

const grid = computed(() => buildCombinedGrid(props.calendar.members))
const days = computed(() =>
  DAYS.map((day, index) => ({
    ...day,
    date: formatDateShort(
      addDaysIso(props.calendar.reference_week.start_date, index),
    ),
  })),
)

const inspection = ref<{ day: number; slot: number } | null>(null)

const inspectedMembers = computed<CalendarMember[]>(() => {
  if (!inspection.value) return []
  return grid.value[inspection.value.day]?.[inspection.value.slot] ?? []
})

const inspectionTitle = computed(() => {
  if (!inspection.value) return ''
  const day = DAYS[inspection.value.day]
  const start = inspection.value.slot * READ_SLOT_MINUTES
  return `${day?.full ?? ''}, ${formatTime(start)}–${formatTime(start + READ_SLOT_MINUTES)}`
})

function cellMembers(day: number, slot: number): CalendarMember[] {
  return grid.value[day]?.[slot] ?? []
}

function openInspection(day: number, slot: number): void {
  if (cellMembers(day, slot).length === 0) return
  inspection.value = { day, slot }
}

function cellLabel(dayIndex: number, slot: number): string {
  const day = DAYS[dayIndex]
  const count = cellMembers(dayIndex, slot).length
  return `${day?.full ?? ''}, ${formatTime(slot * READ_SLOT_MINUTES)}: ${count}`
}
</script>

<template>
  <v-card variant="tonal" class="calendar-card">
    <v-card-item>
      <v-card-title class="text-h6">
        {{ ru.planner.combinedTitle }}
      </v-card-title>
      <v-card-subtitle class="text-wrap">
        {{ ru.planner.combinedSubtitle }}
      </v-card-subtitle>
    </v-card-item>

    <v-card-text class="pt-0">
      <div class="d-flex flex-wrap align-center ga-2 mb-3">
        <v-chip size="small" prepend-icon="mdi-calendar-range">
          {{ formatWeekRange(props.calendar.reference_week.start_date, props.calendar.reference_week.end_date) }}
        </v-chip>
        <v-chip size="small" prepend-icon="mdi-clock-outline">
          {{ props.calendar.viewer_timezone }}
        </v-chip>
      </div>

      <v-alert
        v-if="props.dirty"
        type="warning"
        variant="tonal"
        density="compact"
        class="mb-3"
      >
        {{ ru.planner.combinedDirtyNote }}
      </v-alert>

      <div class="read-grid-wrap">
        <div class="read-grid">
          <div class="corner" aria-hidden="true" />
          <div
            v-for="day in days"
            :key="day.value"
            class="day-head"
            :title="day.full"
          >
            <span class="day-short">{{ day.short }}</span>
            <span class="day-date">{{ day.date }}</span>
          </div>

          <template v-for="(_, slot) in READ_SLOTS_PER_DAY" :key="slot">
            <div
              class="time-label"
              :class="{ 'time-label--hour': slot % 4 === 0 }"
            >
              {{ slot % 4 === 0 ? formatTime(slot * READ_SLOT_MINUTES) : '' }}
            </div>
            <button
              v-for="(day, dayIndex) in DAYS"
              :key="day.value"
              type="button"
              class="read-cell"
              :class="{ 'read-cell--hour': slot % 4 === 0 }"
              :aria-label="cellLabel(dayIndex, slot)"
              @click="openInspection(dayIndex, slot)"
            >
              <span
                v-for="member in cellMembers(dayIndex, slot)"
                :key="member.user_id"
                class="member-dot"
                :style="{ backgroundColor: member.color }"
                :title="memberDisplayName(member)"
              />
            </button>
          </template>
        </div>
      </div>

      <p class="text-caption text-medium-emphasis mt-3 mb-0">
        {{ ru.planner.legendBlank }} · {{ ru.planner.timezoneHint }}
      </p>

      <v-divider class="my-4" />

      <div class="text-subtitle-2 mb-2">{{ ru.planner.membersTitle }}</div>
      <div class="legend-list">
        <div
          v-for="member in props.calendar.members"
          :key="member.user_id"
          class="legend-row"
        >
          <span
            class="legend-dot"
            :style="{ backgroundColor: member.color }"
            aria-hidden="true"
          />
          <span class="text-body-2">{{ memberDisplayName(member) }}</span>
          <span class="text-caption text-medium-emphasis">
            {{ memberRoleLabel(member) }}
          </span>
          <v-chip
            v-if="member.setup_state === 'pending'"
            size="x-small"
            color="warning"
            variant="tonal"
          >
            {{ memberSetupLabel(member) }}
          </v-chip>
        </div>
      </div>
    </v-card-text>
  </v-card>

  <v-dialog
    :model-value="inspection !== null"
    max-width="420"
    @update:model-value="inspection = null"
  >
    <v-card>
      <v-card-title>{{ inspectionTitle }}</v-card-title>
      <v-card-text>
        <p class="text-caption text-medium-emphasis mb-2">
          {{ ru.planner.membersPrefer }}
        </p>
        <div v-if="inspectedMembers.length === 0" class="text-body-2">
          {{ ru.planner.nobodyMarked }}
        </div>
        <v-list v-else density="compact" class="bg-transparent">
          <v-list-item
            v-for="member in inspectedMembers"
            :key="member.user_id"
          >
            <template #prepend>
              <span
                class="legend-dot mr-3"
                :style="{ backgroundColor: member.color }"
              />
            </template>
            <v-list-item-title>{{ memberDisplayName(member) }}</v-list-item-title>
            <v-list-item-subtitle>
              {{ memberRoleLabel(member) }}
            </v-list-item-subtitle>
          </v-list-item>
        </v-list>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="inspection = null">
          {{ ru.planner.close }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.calendar-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.read-grid-wrap {
  width: 100%;
  overflow-x: auto;
}

.read-grid {
  display: grid;
  grid-template-columns: 44px repeat(7, minmax(34px, 1fr));
  min-width: 282px;
}

.corner {
  height: 34px;
}

.day-head {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 34px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.12);
}

.day-short {
  font-size: 0.72rem;
  font-weight: 600;
}

.day-date {
  font-size: 0.58rem;
  color: rgba(var(--v-theme-on-surface), 0.55);
}

.time-label {
  height: 20px;
  font-size: 0.55rem;
  line-height: 20px;
  text-align: right;
  padding-right: 5px;
  color: rgba(var(--v-theme-on-surface), 0.5);
  white-space: nowrap;
}

.time-label--hour {
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}

.read-cell {
  display: flex;
  align-content: center;
  justify-content: center;
  align-items: center;
  flex-wrap: wrap;
  gap: 0;
  height: 20px;
  padding: 1px;
  overflow: hidden;
  border: 0;
  border-top: 1px solid rgba(255, 255, 255, 0.025);
  border-left: 1px solid rgba(255, 255, 255, 0.035);
  background: rgba(255, 255, 255, 0.02);
  cursor: pointer;
}

.read-cell--hour {
  border-top-color: rgba(255, 255, 255, 0.1);
}

.member-dot,
.legend-dot {
  display: inline-block;
  width: 4px;
  height: 4px;
  border-radius: 50%;
}

.legend-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.legend-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.legend-dot {
  width: 10px;
  height: 10px;
  flex: 0 0 auto;
}
</style>
