<script setup lang="ts">
import { computed, ref } from 'vue'

import { mdiCalendarRange, mdiClockOutline } from '@mdi/js'

import { ru } from '../i18n/ru'
import {
  addDaysIso,
  formatDateShort,
  formatWeekRange,
} from '../planner/dates'
import { DAYS, formatTime } from '../planner/model'
import type { BestTimeMember, BestTimeResponse, RankedInterval } from '../types/api'

const props = defineProps<{
  bestTime: BestTimeResponse
  dirty: boolean
}>()

const SHOW_LIMIT = 8
/** Only hide single-member ranges; they are still reachable with "Все". */
const MIN_COUNT = 2
const minCount = ref(MIN_COUNT)
const expanded = ref(false)
const inspected = ref<RankedInterval | null>(null)

const membersById = computed(() => {
  const map = new Map<string, BestTimeMember>()
  for (const member of props.bestTime.members) {
    map.set(member.user_id, member)
  }
  return map
})

const allRows = computed(() =>
  props.bestTime.intervals.map((interval, index) => ({
    ...interval,
    rank: index + 1,
    dayLabel: DAYS[interval.day]?.short ?? '',
    dateLabel: formatDateShort(
      addDaysIso(props.bestTime.reference_week.start_date, interval.day),
    ),
  })),
)

const filteredRows = computed(() =>
  minCount.value > 1
    ? allRows.value.filter((row) => row.count >= minCount.value)
    : allRows.value,
)

const visibleRows = computed(() =>
  expanded.value ? filteredRows.value : filteredRows.value.slice(0, SHOW_LIMIT),
)

const hiddenCount = computed(() => filteredRows.value.length - visibleRows.value.length)

const hasSingleMemberRows = computed(() =>
  allRows.value.some((row) => row.count < MIN_COUNT),
)

const inspectedMembers = computed<BestTimeMember[]>(() => {
  if (!inspected.value) return []
  return inspected.value.members
    .map((id) => membersById.value.get(id))
    .filter((member): member is BestTimeMember => member !== undefined)
})

const inspectedTitle = computed(() => {
  const interval = inspected.value
  if (!interval) return ''
  const day = DAYS[interval.day]?.full ?? ''
  return `${day}, ${formatTime(interval.start_minute)}–${formatTime(interval.end_minute)}`
})

function memberName(member: BestTimeMember): string {
  const first = member.first_name?.trim() ?? ''
  const last = member.last_name?.trim() ?? ''
  if (first && last) return `${first} ${last}`
  if (first) return first
  if (member.username) return `@${member.username}`
  return ru.common.memberFallback
}

function toggleExpanded(): void {
  expanded.value = !expanded.value
}
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
        <v-chip size="small" :prepend-icon="mdiCalendarRange">
          {{ formatWeekRange(props.bestTime.reference_week.start_date, props.bestTime.reference_week.end_date) }}
        </v-chip>
        <v-chip size="small" :prepend-icon="mdiClockOutline">
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
        v-else-if="allRows.length === 0"
        type="info"
        variant="tonal"
        density="comfortable"
      >
        {{ ru.planner.bestEmpty }}
      </v-alert>

      <template v-else>
        <div v-if="hasSingleMemberRows" class="mb-3">
          <v-btn-toggle
            :model-value="minCount"
            mandatory
            density="comfortable"
            variant="outlined"
            divided
            @update:model-value="minCount = Number($event) || 1"
          >
            <v-btn :value="MIN_COUNT" size="small">
              {{ ru.planner.bestFilterAtLeastTwo }}
            </v-btn>
            <v-btn :value="1" size="small">
              {{ ru.planner.bestFilterAll }}
            </v-btn>
          </v-btn-toggle>
        </div>

        <v-alert
          v-if="filteredRows.length === 0"
          type="info"
          variant="tonal"
          density="comfortable"
        >
          {{ ru.planner.bestNoOverlap }}
        </v-alert>

        <div v-else class="rank-list">
          <button
            v-for="row in visibleRows"
            :key="`${row.start}-${row.end}`"
            type="button"
            class="rank-row"
            @click="inspected = row"
          >
            <span class="rank-number">{{ row.rank }}</span>
            <span class="rank-time">
              <span class="text-body-medium font-weight-medium">
                {{ row.dayLabel }}, {{ row.dateLabel }}
              </span>
              <span class="text-body-small text-medium-emphasis">
                {{ formatTime(row.start_minute) }}–{{ formatTime(row.end_minute) }}
              </span>
            </span>
            <v-chip size="small" color="primary" variant="tonal">
              {{ ru.planner.preferredBy(row.count) }}
            </v-chip>
          </button>
        </div>

        <div v-if="hiddenCount > 0 || expanded" class="d-flex justify-center mt-3">
          <v-btn size="small" variant="text" color="primary" @click="toggleExpanded">
            {{ expanded ? ru.planner.bestShowLess : ru.planner.bestShowAll(filteredRows.length) }}
          </v-btn>
        </div>
      </template>

      <p class="text-body-small text-medium-emphasis mt-4 mb-0">
        {{ ru.planner.bestNote }}
      </p>
    </v-card-text>
  </v-card>

  <v-dialog
    :model-value="inspected !== null"
    max-width="420"
    @update:model-value="inspected = null"
  >
    <v-card>
      <v-card-title>{{ inspectedTitle }}</v-card-title>
      <v-card-text>
        <p class="text-body-small text-medium-emphasis mb-2">
          {{ ru.planner.bestMembersTitle }}
        </p>
        <v-list v-if="inspectedMembers.length > 0" density="compact" class="bg-transparent">
          <v-list-item
            v-for="member in inspectedMembers"
            :key="member.user_id"
          >
            <template #prepend>
              <span
                class="member-dot mr-3"
                :style="{ backgroundColor: member.color }"
                aria-hidden="true"
              />
            </template>
            <v-list-item-title>{{ memberName(member) }}</v-list-item-title>
          </v-list-item>
        </v-list>
        <div v-else class="text-body-medium">
          {{ ru.planner.nobodyMarked }}
        </div>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="inspected = null">
          {{ ru.planner.close }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.calendar-card {
  border: 1px solid var(--app-border);
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
  width: 100%;
  padding: 8px 0;
  border: 0;
  border-top: 1px solid var(--app-border);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.rank-row:active {
  opacity: 0.75;
}

.rank-row:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: 2px;
}

.rank-number {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: color-mix(in srgb, rgb(var(--v-theme-primary)) 16%, transparent);
  color: rgb(var(--v-theme-primary));
  font-size: 0.72rem;
  font-weight: 700;
}

.rank-time {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-width: 0;
}

.member-dot {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
</style>
