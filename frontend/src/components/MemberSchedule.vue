<script setup lang="ts">
import { computed } from 'vue'

import { mdiArrowLeft, mdiCalendarRange, mdiClockOutline, mdiEarth } from '@mdi/js'

import { ru } from '../i18n/ru'
import { segmentsForDay } from '../planner/combined'
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
import { formatOffset } from '../timezones'
import type { MemberCalendarResponse } from '../types/api'

const props = defineProps<{
  response: MemberCalendarResponse
}>()

const emit = defineEmits<{
  back: []
}>()

const member = computed(() => props.response.member)
const pending = computed(() => member.value.setup_state === 'pending')
const empty = computed(
  () => !pending.value && member.value.segments.length === 0,
)

/**
 * The member's own zone is shown because schedules are converted into the
 * viewer's zone; without it, "21:00" is ambiguous when members live apart.
 */
const memberTimezone = computed(() => member.value.timezone)
const memberOffset = computed(() =>
  memberTimezone.value ? formatOffset(memberTimezone.value) : '',
)

const days = computed(() =>
  DAYS.map((day, index) => ({
    ...day,
    date: formatDateShort(
      addDaysIso(props.response.reference_week.start_date, index),
    ),
    segments: segmentsForDay(member.value, index),
  })),
)
</script>

<template>
  <v-card variant="tonal" class="calendar-card">
    <v-card-item>
      <template #prepend>
        <span
          class="member-swatch"
          :style="{ backgroundColor: member.color }"
          aria-hidden="true"
        />
      </template>
      <v-card-title class="text-title-large">
        {{ memberDisplayName(member) }}
      </v-card-title>
      <v-card-subtitle>
        {{ memberRoleLabel(member) }} · {{ memberSetupLabel(member) }}
      </v-card-subtitle>
    </v-card-item>

    <v-card-text class="pt-0">
      <div class="d-flex flex-wrap align-center ga-2 mb-3">
        <v-btn
          size="small"
          variant="text"
          :prepend-icon="mdiArrowLeft"
          @click="emit('back')"
        >
          {{ ru.planner.backToMembers }}
        </v-btn>
        <v-spacer />
        <v-chip size="small" :prepend-icon="mdiCalendarRange">
          {{ formatWeekRange(response.reference_week.start_date, response.reference_week.end_date) }}
        </v-chip>
        <v-chip size="small" :prepend-icon="mdiClockOutline">
          {{ response.viewer_timezone }}
        </v-chip>
      </div>

      <v-chip
        v-if="memberTimezone"
        size="small"
        variant="outlined"
        class="mb-3"
        :prepend-icon="mdiEarth"
      >
        {{ ru.planner.memberTimezone }}: {{ memberTimezone }}
        <template v-if="memberOffset"> · {{ memberOffset }}</template>
      </v-chip>

      <v-alert
        v-if="pending"
        type="info"
        variant="tonal"
        density="comfortable"
        class="mb-3"
      >
        {{ ru.planner.pendingNoCalendar }}
      </v-alert>

      <v-alert
        v-else-if="empty"
        type="info"
        variant="tonal"
        density="comfortable"
        class="mb-3"
      >
        {{ ru.planner.noSegments }}
      </v-alert>

      <div v-else class="day-list">
        <div
          v-for="day in days"
          :key="day.value"
          class="day-row"
        >
          <div class="day-label">
            <span class="font-weight-medium">{{ day.short }}</span>
            <span class="text-body-small text-medium-emphasis">{{ day.date }}</span>
          </div>
          <div class="day-segments">
            <span
              v-for="segment in day.segments"
              :key="`${segment.start}-${segment.end}`"
              class="segment-chip"
              :style="{ backgroundColor: member.color }"
            >
              {{ formatTime(segment.start_minute) }}–{{ formatTime(segment.end_minute) }}
            </span>
            <span
              v-if="day.segments.length === 0"
              class="text-body-small text-medium-emphasis"
            >
              {{ ru.planner.noSegments }}
            </span>
          </div>
        </div>
      </div>

      <p class="text-body-small text-medium-emphasis mt-3 mb-0">
        {{ ru.planner.legendBlank }} · {{ ru.planner.timezoneHint }}
      </p>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.calendar-card {
  border: 1px solid var(--app-border);
}

.member-swatch {
  display: inline-block;
  width: 18px;
  height: 18px;
  border-radius: 6px;
}

.day-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.day-row {
  display: grid;
  grid-template-columns: 74px 1fr;
  gap: 8px;
  align-items: start;
  padding: 6px 0;
  border-top: 1px solid var(--app-border);
}

.day-label {
  display: flex;
  flex-direction: column;
  line-height: 1.2;
}

.day-segments {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  min-height: 24px;
  align-items: center;
}

.segment-chip {
  display: inline-block;
  padding: 2px 7px;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 600;
  color: #0b0f19;
}
</style>
