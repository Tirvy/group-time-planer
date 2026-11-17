<script setup lang="ts">
import { computed } from 'vue'

import { ru } from '../i18n/ru'
import {
  addDaysIso,
  formatDateShort,
  formatWeekRange,
} from '../planner/dates'
import { DAYS, formatTime } from '../planner/model'
import type { BestTimeResponse } from '../types/api'

const props = defineProps<{
  bestTime: BestTimeResponse
  dirty: boolean
}>()

const rows = computed(() =>
  props.bestTime.intervals.map((interval, index) => ({
    ...interval,
    rank: index + 1,
    dayLabel: DAYS[interval.day]?.short ?? '',
    dateLabel: formatDateShort(
      addDaysIso(props.bestTime.reference_week.start_date, interval.day),
    ),
  })),
)
</script>

<template>
  <v-card variant="tonal" class="calendar-card">
    <v-card-item>
      <v-card-title class="text-title-large">
        {{ ru.planner.bestTimeTitle }}
      </v-card-title>
      <v-card-subtitle class="text-wrap">
        {{ ru.planner.bestTimeSubtitle }}
      </v-card-subtitle>
    </v-card-item>

    <v-card-text class="pt-0">
      <div class="d-flex flex-wrap align-center ga-2 mb-3">
        <v-chip size="small" prepend-icon="mdi-calendar-range">
          {{ formatWeekRange(props.bestTime.reference_week.start_date, props.bestTime.reference_week.end_date) }}
        </v-chip>
        <v-chip size="small" prepend-icon="mdi-clock-outline">
          {{ props.bestTime.viewer_timezone }}
        </v-chip>
        <v-chip size="small" color="success" variant="tonal">
          {{ ru.planner.bestConfigured }}:
          {{ props.bestTime.configured_member_count }} {{ ru.common.of }}
          {{ props.bestTime.total_member_count }}
        </v-chip>
        <v-chip
          v-if="props.bestTime.pending_member_count > 0"
          size="small"
          color="warning"
          variant="tonal"
        >
          {{ ru.planner.bestPending }}:
          {{ props.bestTime.pending_member_count }}
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

      <v-alert
        v-if="props.bestTime.configured_member_count === 0"
        type="info"
        variant="tonal"
        density="comfortable"
      >
        {{ ru.planner.bestNoConfigured }}
      </v-alert>
      <v-alert
        v-else-if="rows.length === 0"
        type="info"
        variant="tonal"
        density="comfortable"
      >
        {{ ru.planner.bestEmpty }}
      </v-alert>

      <div v-else class="rank-list">
        <div
          v-for="row in rows"
          :key="`${row.start}-${row.end}`"
          class="rank-row"
        >
          <span class="rank-number">{{ row.rank }}</span>
          <div class="rank-time">
            <div class="text-body-medium font-weight-medium">
              {{ row.dayLabel }}, {{ row.dateLabel }}
            </div>
            <div class="text-body-small text-medium-emphasis">
              {{ formatTime(row.start_minute) }}–{{ formatTime(row.end_minute) }}
            </div>
          </div>
          <v-chip size="small" color="primary" variant="tonal">
            {{ ru.planner.preferredBy(row.count) }}
          </v-chip>
        </div>
      </div>

      <p class="text-body-small text-medium-emphasis mt-4 mb-0">
        {{ ru.planner.bestNote }}
      </p>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.calendar-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.rank-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.rank-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
}

.rank-number {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: rgba(77, 208, 225, 0.16);
  color: rgb(var(--v-theme-primary));
  font-size: 0.72rem;
  font-weight: 700;
}

.rank-time {
  flex: 1 1 auto;
  min-width: 0;
}
</style>
