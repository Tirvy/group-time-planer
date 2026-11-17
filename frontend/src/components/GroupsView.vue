<script setup lang="ts">
import { defineAsyncComponent, onMounted, ref, watch } from 'vue'

import { mdiAccountGroupOutline, mdiChevronRight, mdiDownload, mdiRefresh } from '@mdi/js'

import {
  fetchBestTime,
  fetchGroupCalendar,
  fetchGroups,
  fetchMemberCalendar,
  fetchPreferences,
  fetchTimetablePng,
  savePreferences,
} from '../api/client'
import { describeError } from '../api/errors'
import { useAsyncResource } from '../composables/useAsyncResource'
import { ru } from '../i18n/ru'
import {
  memberDisplayName,
  memberRoleLabel,
  memberSetupLabel,
} from '../planner/names'
import type { CalendarMember, GroupDto, PreferenceInterval } from '../types/api'
import AsyncState from './AsyncState.vue'
import CalendarEditor from './CalendarEditor.vue'

// Read-only views are only needed after the user opens their tab; loading them
// lazily keeps the initial Mini App bundle focused on the editor.
const BestTimeView = defineAsyncComponent(() => import('./BestTimeView.vue'))
const CombinedTimetable = defineAsyncComponent(
  () => import('./CombinedTimetable.vue'),
)
const MemberSchedule = defineAsyncComponent(
  () => import('./MemberSchedule.vue'),
)

const props = defineProps<{
  initData: string
}>()

const groups = ref<GroupDto[]>([])
const selected = ref<GroupDto | null>(null)
const loadingGroups = ref(true)
const groupsError = ref<string | null>(null)
const saving = ref(false)
const saveError = ref<string | null>(null)
const dirty = ref(false)
const pendingGroup = ref<GroupDto | null>(null)
const discardOpen = ref(false)
const snackbar = ref(false)

type GroupTab = 'mine' | 'combined' | 'members' | 'best'
const activeTab = ref<GroupTab>('mine')

const {
  data: preferences,
  loading: loadingPreferences,
  error: preferencesError,
  run: runPreferences,
  retry: retryPreferences,
  reset: resetPreferences,
} = useAsyncResource(
  (groupId) => fetchPreferences(props.initData, groupId),
  ru.planner.loadError,
)
const {
  data: calendar,
  loading: calendarLoading,
  error: calendarError,
  run: runCalendar,
  retry: retryCalendar,
  reset: resetCalendar,
} = useAsyncResource(
  (groupId) => fetchGroupCalendar(props.initData, groupId),
  ru.planner.calendarLoadError,
)
const {
  data: bestTime,
  loading: bestTimeLoading,
  error: bestTimeError,
  run: runBestTime,
  retry: retryBestTime,
  reset: resetBestTime,
} = useAsyncResource(
  (groupId) => fetchBestTime(props.initData, groupId),
  ru.planner.bestLoadError,
)
const {
  data: memberCalendar,
  loading: memberLoading,
  error: memberError,
  run: runMember,
  retry: retryMemberResource,
  reset: resetMember,
} = useAsyncResource(
  (key) => {
    const separator = key.indexOf(':')
    const groupId = key.slice(0, separator)
    const memberId = key.slice(separator + 1)
    return fetchMemberCalendar(props.initData, groupId, memberId)
  },
  ru.planner.memberScheduleError,
)

const selectedMember = ref<CalendarMember | null>(null)
const pngLoading = ref(false)
const pngError = ref<string | null>(null)

onMounted(loadGroups)

watch(activeTab, (tab) => {
  // Payloads are cached per group and invalidated by a save, so switching tabs
  // must not trigger another expensive calendar calculation.
  if (tab === 'combined' || tab === 'members') {
    ensureCalendar()
  } else if (tab === 'best') {
    ensureBestTime()
  }
})

async function loadGroups(): Promise<void> {
  loadingGroups.value = true
  groupsError.value = null

  try {
    const response = await fetchGroups(props.initData)
    groups.value = response.groups

    if (response.groups.length === 1) {
      await selectGroup(response.groups[0]!, false)
      return
    }

    const requestedGroup = new URLSearchParams(window.location.search).get(
      'group',
    )
    const requested = requestedGroup
      ? response.groups.find((group) => group.id === requestedGroup)
      : undefined
    if (requested) {
      await selectGroup(requested, false)
      return
    }

    if (selected.value) {
      const refreshed = response.groups.find(
        (group) => group.id === selected.value?.id,
      )
      selected.value = refreshed ?? null
    }
  } catch (error) {
    groupsError.value = describeError(error, ru.planner.groupsLoadError)
  } finally {
    loadingGroups.value = false
  }
}

async function selectGroup(group: GroupDto, checkDirty = true): Promise<void> {
  if (
    checkDirty &&
    dirty.value &&
    selected.value &&
    group.id !== selected.value.id
  ) {
    pendingGroup.value = group
    discardOpen.value = true
    return
  }

  selected.value = group
  preferences.value = null
  saveError.value = null
  dirty.value = false
  activeTab.value = 'mine'
  resetPreferences()
  resetCalendar()
  resetBestTime()
  resetMember()
  selectedMember.value = null
  pngLoading.value = false
  pngError.value = null
  await runPreferences(group.id)
}

function requestGroupChange(id: string | null): void {
  if (!id) return
  const group = groups.value.find((candidate) => candidate.id === id)
  if (group) {
    void selectGroup(group)
  }
}

function confirmGroupChange(): void {
  discardOpen.value = false
  const group = pendingGroup.value
  pendingGroup.value = null
  if (group) {
    void selectGroup(group, false)
  }
}

function cancelGroupChange(): void {
  discardOpen.value = false
  pendingGroup.value = null
}

function retryPreferencesLoad(): void {
  const group = selected.value
  if (group) void retryPreferences(group.id)
}

function ensureCalendar(force = false): void {
  const group = selected.value
  if (group) void runCalendar(group.id, force)
}

function ensureBestTime(force = false): void {
  const group = selected.value
  if (group) void runBestTime(group.id, force)
}

function retryCalendarLoad(): void {
  const group = selected.value
  if (group) void retryCalendar(group.id)
}

function retryBestTimeLoad(): void {
  const group = selected.value
  if (group) void retryBestTime(group.id)
}

async function downloadPng(): Promise<void> {
  const group = selected.value
  if (!group || pngLoading.value) return

  pngLoading.value = true
  pngError.value = null

  try {
    const blob = await fetchTimetablePng(props.initData, group.id)
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = 'timetable.png'
    document.body.appendChild(link)
    link.click()
    link.remove()
    window.setTimeout(() => URL.revokeObjectURL(url), 2000)
  } catch (error) {
    pngError.value = describeError(error, ru.planner.pngError)
  } finally {
    pngLoading.value = false
  }
}

function memberKey(groupId: string, memberId: string): string {
  return `${groupId}:${memberId}`
}

function selectMember(member: CalendarMember): void {
  const group = selected.value
  if (!group) return

  selectedMember.value = member
  void runMember(memberKey(group.id, member.user_id))
}

function backToMembers(): void {
  resetMember()
  selectedMember.value = null
}

function retryMember(): void {
  const group = selected.value
  const member = selectedMember.value
  if (group && member) {
    void retryMemberResource(memberKey(group.id, member.user_id))
  }
}

async function handleSave(intervals: PreferenceInterval[]): Promise<void> {
  const group = selected.value
  if (!group) return

  saving.value = true
  saveError.value = null

  try {
    const response = await savePreferences(props.initData, group.id, intervals)
    const updated = { ...group, setup_completed: response.setup_completed }
    groups.value = groups.value.map((candidate) =>
      candidate.id === group.id ? updated : candidate,
    )

    if (selected.value?.id === group.id) {
      preferences.value = response
      selected.value = updated
      // Saved data invalidates every derived view for this group.
      resetCalendar()
      resetBestTime()
      resetMember()
      selectedMember.value = null
      pngError.value = null
    }
    snackbar.value = true
  } catch (error) {
    saveError.value = describeError(error, ru.planner.saveError)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div>
    <div
      v-if="loadingGroups"
      class="d-flex flex-column align-center py-10"
      role="status"
      aria-live="polite"
    >
      <v-progress-circular indeterminate color="primary" />
      <p class="text-body-medium text-medium-emphasis mt-3 mb-0">
        {{ ru.planner.loadingGroups }}
      </p>
    </div>

    <v-card v-else-if="groupsError" variant="tonal" class="state-card">
      <v-card-text>
        <v-alert type="error" variant="tonal" density="comfortable" role="alert">
          {{ groupsError }}
        </v-alert>
      </v-card-text>
      <v-card-actions class="px-4 pb-4">
        <v-spacer />
        <v-btn color="primary" variant="flat" :prepend-icon="mdiRefresh" @click="loadGroups">
          {{ ru.planner.retry }}
        </v-btn>
      </v-card-actions>
    </v-card>

    <v-card v-else-if="groups.length === 0" variant="tonal" class="state-card">
      <v-card-item>
        <template #prepend>
          <v-icon color="primary" size="30" :icon="mdiAccountGroupOutline" />
        </template>
        <v-card-title>{{ ru.planner.noGroupsTitle }}</v-card-title>
        <v-card-subtitle class="text-wrap">
          {{ ru.planner.noGroupsBody }}
        </v-card-subtitle>
      </v-card-item>
    </v-card>

    <template v-else>
      <v-select
        v-if="groups.length > 1"
        :model-value="selected?.id ?? null"
        :items="groups"
        item-title="name"
        item-value="id"
        :label="ru.planner.groupLabel"
        density="comfortable"
        variant="outlined"
        class="mb-3"
        @update:model-value="requestGroupChange"
      />

      <v-alert
        v-if="!selected"
        type="info"
        variant="tonal"
        density="comfortable"
        class="mb-3"
      >
        {{ ru.planner.chooseGroup }}
      </v-alert>

      <template v-if="selected">
        <v-card variant="tonal" class="state-card mb-4">
          <v-card-item>
            <template #prepend>
              <span
                class="group-color"
                :style="{ backgroundColor: selected.color }"
                aria-hidden="true"
              />
            </template>
            <v-card-title>{{ selected.name }}</v-card-title>
          </v-card-item>
        </v-card>

        <div class="d-flex align-center flex-wrap ga-2 mb-3">
          <v-btn
            size="small"
            variant="tonal"
            color="primary"
            :prepend-icon="mdiDownload"
            :loading="pngLoading"
            @click="downloadPng"
          >
            {{ pngLoading ? ru.planner.downloadingPng : ru.planner.downloadPng }}
          </v-btn>
          <span v-if="pngError" class="text-body-small error-text">
            {{ pngError }}
          </span>
        </div>

        <v-tabs
          v-model="activeTab"
          color="primary"
          density="comfortable"
          grow
          class="planner-tabs mb-3"
        >
          <v-tab value="mine">{{ ru.planner.tabMine }}</v-tab>
          <v-tab value="combined">{{ ru.planner.tabCombined }}</v-tab>
          <v-tab value="members">{{ ru.planner.tabMembers }}</v-tab>
          <v-tab value="best">{{ ru.planner.tabBest }}</v-tab>
        </v-tabs>

        <div v-show="activeTab === 'mine'">
          <AsyncState
            :loading="loadingPreferences"
            :error="preferencesError"
            :loading-text="ru.planner.loadingPreferences"
            @retry="retryPreferencesLoad"
          >
            <CalendarEditor
              v-if="preferences"
              :key="selected.id"
              :group="selected"
              :preferences="preferences"
              :saving="saving"
              :save-error="saveError"
              @save="handleSave"
              @dirty="dirty = $event"
            />
          </AsyncState>
        </div>

        <template v-if="activeTab === 'combined'">
          <AsyncState
            :loading="calendarLoading"
            :error="calendarError"
            :loading-text="ru.planner.loadingCalendar"
            @retry="retryCalendarLoad"
          >
            <CombinedTimetable v-if="calendar" :calendar="calendar" :dirty="dirty" />
          </AsyncState>
        </template>

        <template v-if="activeTab === 'members'">
          <AsyncState
            :loading="calendarLoading"
            :error="calendarError"
            :loading-text="ru.planner.loadingCalendar"
            @retry="retryCalendarLoad"
          >
            <template v-if="calendar">
              <v-card v-if="!selectedMember" variant="tonal" class="state-card">
                <v-card-item>
                  <v-card-title class="text-title-large">
                    {{ ru.planner.membersTitle }}
                  </v-card-title>
                  <v-card-subtitle>
                    {{ ru.planner.timezoneHint }}
                  </v-card-subtitle>
                </v-card-item>
                <v-list class="bg-transparent">
                  <v-list-item
                    v-for="member in calendar.members"
                    :key="member.user_id"
                    @click="selectMember(member)"
                  >
                    <template #prepend>
                      <span
                        class="member-color"
                        :style="{ backgroundColor: member.color }"
                        aria-hidden="true"
                      />
                    </template>
                    <v-list-item-title>{{ memberDisplayName(member) }}</v-list-item-title>
                    <v-list-item-subtitle>
                      {{ memberRoleLabel(member) }} · {{ memberSetupLabel(member) }}
                    </v-list-item-subtitle>
                    <template #append>
                      <v-icon :icon="mdiChevronRight" />
                    </template>
                  </v-list-item>
                </v-list>
              </v-card>

              <AsyncState
                v-else
                :loading="memberLoading"
                :error="memberError"
                :loading-text="ru.planner.memberScheduleLoading"
                @retry="retryMember"
              >
                <template #error-actions>
                  <v-btn variant="text" @click="backToMembers">
                    {{ ru.planner.backToMembers }}
                  </v-btn>
                </template>
                <MemberSchedule
                  v-if="memberCalendar"
                  :response="memberCalendar"
                  @back="backToMembers"
                />
              </AsyncState>
            </template>
          </AsyncState>
        </template>

        <template v-if="activeTab === 'best'">
          <AsyncState
            :loading="bestTimeLoading"
            :error="bestTimeError"
            :loading-text="ru.planner.loadingBest"
            @retry="retryBestTimeLoad"
          >
            <BestTimeView v-if="bestTime" :best-time="bestTime" :dirty="dirty" />
          </AsyncState>
        </template>
      </template>
    </template>

    <v-dialog v-model="discardOpen" max-width="420">
      <v-card>
        <v-card-title>{{ ru.planner.discardTitle }}</v-card-title>
        <v-card-text>{{ ru.planner.discardBody }}</v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="cancelGroupChange">
            {{ ru.planner.stay }}
          </v-btn>
          <v-btn color="warning" variant="flat" @click="confirmGroupChange">
            {{ ru.planner.leave }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-snackbar v-model="snackbar" color="success" :timeout="2500">
      {{ ru.planner.saved }}
    </v-snackbar>
  </div>
</template>

<style scoped>
.state-card {
  border: 1px solid var(--app-border);
}

.planner-tabs {
  border-bottom: 1px solid var(--app-border);
}

.planner-tabs :deep(.v-tab) {
  min-width: 0;
  padding-inline: 6px;
  font-size: 0.75rem;
  letter-spacing: 0;
  text-transform: none;
}

.error-text {
  color: rgb(var(--v-theme-error));
}

.group-color {
  display: inline-block;
  width: 18px;
  height: 18px;
  border-radius: 6px;
}

.member-color {
  display: inline-block;
  width: 14px;
  height: 14px;
  border-radius: 50%;
}
</style>
