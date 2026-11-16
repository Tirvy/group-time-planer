<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'

import {
  ApiError,
  fetchGroupCalendar,
  fetchGroups,
  fetchMemberCalendar,
  fetchPreferences,
  savePreferences,
} from '../api/client'
import { ru } from '../i18n/ru'
import {
  memberDisplayName,
  memberRoleLabel,
  memberSetupLabel,
} from '../planner/names'
import type {
  CalendarMember,
  GroupCalendarResponse,
  GroupDto,
  MemberCalendarResponse,
  PreferenceInterval,
  PreferencesResponse,
} from '../types/api'
import CalendarEditor from './CalendarEditor.vue'
import CombinedTimetable from './CombinedTimetable.vue'
import MemberSchedule from './MemberSchedule.vue'

const props = defineProps<{
  initData: string
}>()

const errorMessages: Record<string, string> = { ...ru.errors }

const groups = ref<GroupDto[]>([])
const selected = ref<GroupDto | null>(null)
const preferences = ref<PreferencesResponse | null>(null)
const loadingGroups = ref(true)
const loadingPreferences = ref(false)
const groupsError = ref<string | null>(null)
const preferencesError = ref<string | null>(null)
const saving = ref(false)
const saveError = ref<string | null>(null)
const dirty = ref(false)
const pendingGroup = ref<GroupDto | null>(null)
const discardOpen = ref(false)
const snackbar = ref(false)
let preferencesRequest = 0

type GroupTab = 'mine' | 'combined' | 'members'
const activeTab = ref<GroupTab>('mine')
const calendar = ref<GroupCalendarResponse | null>(null)
const calendarLoading = ref(false)
const calendarError = ref<string | null>(null)
let calendarRequest = 0
const selectedMember = ref<CalendarMember | null>(null)
const memberCalendar = ref<MemberCalendarResponse | null>(null)
const memberLoading = ref(false)
const memberError = ref<string | null>(null)
let memberRequest = 0

onMounted(loadGroups)

watch(activeTab, (tab) => {
  if (tab !== 'mine') {
    void ensureCalendar(true)
  }
})

function messageFor(error: unknown, fallback: string): string {
  if (error instanceof ApiError) {
    return errorMessages[error.code] ?? fallback
  }
  return fallback
}

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

    if (selected.value) {
      const refreshed = response.groups.find(
        (group) => group.id === selected.value?.id,
      )
      selected.value = refreshed ?? null
    }
  } catch (error) {
    groupsError.value = messageFor(error, ru.planner.groupsLoadError)
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
  preferencesError.value = null
  saveError.value = null
  dirty.value = false
  activeTab.value = 'mine'
  calendar.value = null
  calendarError.value = null
  calendarLoading.value = false
  calendarRequest += 1
  selectedMember.value = null
  memberCalendar.value = null
  memberError.value = null
  memberLoading.value = false
  memberRequest += 1
  await loadPreferences(group)
}

async function loadPreferences(group: GroupDto): Promise<void> {
  const requestId = ++preferencesRequest
  loadingPreferences.value = true
  preferencesError.value = null

  try {
    const response = await fetchPreferences(props.initData, group.id)
    if (
      requestId !== preferencesRequest ||
      selected.value?.id !== group.id
    ) {
      return
    }
    preferences.value = response
  } catch (error) {
    if (
      requestId === preferencesRequest &&
      selected.value?.id === group.id
    ) {
      preferencesError.value = messageFor(error, ru.planner.loadError)
    }
  } finally {
    if (requestId === preferencesRequest) {
      loadingPreferences.value = false
    }
  }
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

function retryPreferences(): void {
  if (selected.value) {
    void loadPreferences(selected.value)
  }
}

async function ensureCalendar(force = false): Promise<void> {
  const group = selected.value
  if (!group) return
  if (!force && calendar.value?.group.id === group.id) return
  if (calendarLoading.value) return

  const requestId = ++calendarRequest
  calendarLoading.value = true
  calendarError.value = null

  try {
    const response = await fetchGroupCalendar(props.initData, group.id)
    if (requestId !== calendarRequest || selected.value?.id !== group.id) {
      return
    }
    calendar.value = response
  } catch (error) {
    if (requestId === calendarRequest && selected.value?.id === group.id) {
      calendarError.value = messageFor(error, ru.planner.calendarLoadError)
    }
  } finally {
    if (requestId === calendarRequest) {
      calendarLoading.value = false
    }
  }
}

function retryCalendar(): void {
  calendar.value = null
  void ensureCalendar(true)
}

async function selectMember(member: CalendarMember): Promise<void> {
  const group = selected.value
  if (!group) return

  selectedMember.value = member
  memberCalendar.value = null
  memberError.value = null
  memberLoading.value = true
  const requestId = ++memberRequest

  try {
    const response = await fetchMemberCalendar(
      props.initData,
      group.id,
      member.user_id,
    )
    if (
      requestId !== memberRequest ||
      selected.value?.id !== group.id ||
      selectedMember.value?.user_id !== member.user_id
    ) {
      return
    }
    memberCalendar.value = response
  } catch (error) {
    if (requestId === memberRequest) {
      memberError.value = messageFor(error, ru.planner.memberScheduleError)
    }
  } finally {
    if (requestId === memberRequest) {
      memberLoading.value = false
    }
  }
}

function backToMembers(): void {
  memberRequest += 1
  selectedMember.value = null
  memberCalendar.value = null
  memberError.value = null
  memberLoading.value = false
}

function retryMember(): void {
  if (selectedMember.value) {
    void selectMember(selectedMember.value)
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
      calendar.value = null
      selectedMember.value = null
      memberCalendar.value = null
      memberRequest += 1
    }
    snackbar.value = true
  } catch (error) {
    saveError.value = messageFor(error, ru.planner.saveError)
  } finally {
    saving.value = false
  }
}

function roleLabel(group: GroupDto): string {
  return group.role === 'owner' ? ru.planner.owner : ru.planner.member
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
      <p class="text-body-2 text-medium-emphasis mt-3 mb-0">
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
        <v-btn color="primary" variant="flat" prepend-icon="mdi-refresh" @click="loadGroups">
          {{ ru.planner.retry }}
        </v-btn>
      </v-card-actions>
    </v-card>

    <v-card v-else-if="groups.length === 0" variant="tonal" class="state-card">
      <v-card-item>
        <template #prepend>
          <v-icon color="primary" size="30">mdi-account-group-outline</v-icon>
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
            <v-card-subtitle>
              {{ roleLabel(selected) }} ·
              {{ ru.planner.participants(selected.member_count) }}
            </v-card-subtitle>
          </v-card-item>
          <v-card-text class="pt-1">
            <v-chip
              size="small"
              :color="selected.setup_completed ? 'success' : 'warning'"
              variant="tonal"
            >
              {{
                selected.setup_completed
                  ? ru.planner.setupCompletedShort
                  : ru.planner.setupPendingShort
              }}
            </v-chip>
          </v-card-text>
        </v-card>

        <v-btn-toggle
          v-model="activeTab"
          mandatory
          divided
          color="primary"
          density="comfortable"
          class="mb-3 flex-wrap"
        >
          <v-btn value="mine" prepend-icon="mdi-calendar-edit">
            {{ ru.planner.tabMine }}
          </v-btn>
          <v-btn value="combined" prepend-icon="mdi-calendar-week">
            {{ ru.planner.tabCombined }}
          </v-btn>
          <v-btn value="members" prepend-icon="mdi-account-group-outline">
            {{ ru.planner.tabMembers }}
          </v-btn>
        </v-btn-toggle>

        <div v-show="activeTab === 'mine'">
          <div
            v-if="loadingPreferences"
            class="d-flex flex-column align-center py-8"
            role="status"
            aria-live="polite"
          >
            <v-progress-circular indeterminate color="primary" size="28" />
            <p class="text-body-2 text-medium-emphasis mt-3 mb-0">
              {{ ru.planner.loadingPreferences }}
            </p>
          </div>

          <v-card v-else-if="preferencesError" variant="tonal" class="state-card">
            <v-card-text>
              <v-alert type="error" variant="tonal" density="comfortable" role="alert">
                {{ preferencesError }}
              </v-alert>
            </v-card-text>
            <v-card-actions class="px-4 pb-4">
              <v-spacer />
              <v-btn color="primary" variant="flat" prepend-icon="mdi-refresh" @click="retryPreferences">
                {{ ru.planner.retry }}
              </v-btn>
            </v-card-actions>
          </v-card>

          <CalendarEditor
            v-else-if="preferences"
            :key="selected.id"
            :group="selected"
            :preferences="preferences"
            :saving="saving"
            :save-error="saveError"
            @save="handleSave"
            @dirty="dirty = $event"
          />
        </div>

        <template v-if="activeTab === 'combined'">
          <div
            v-if="calendarLoading"
            class="d-flex flex-column align-center py-8"
            role="status"
            aria-live="polite"
          >
            <v-progress-circular indeterminate color="primary" size="28" />
            <p class="text-body-2 text-medium-emphasis mt-3 mb-0">
              {{ ru.planner.loadingCalendar }}
            </p>
          </div>

          <v-card v-else-if="calendarError" variant="tonal" class="state-card">
            <v-card-text>
              <v-alert type="error" variant="tonal" density="comfortable" role="alert">
                {{ calendarError }}
              </v-alert>
            </v-card-text>
            <v-card-actions class="px-4 pb-4">
              <v-spacer />
              <v-btn color="primary" variant="flat" prepend-icon="mdi-refresh" @click="retryCalendar">
                {{ ru.planner.retry }}
              </v-btn>
            </v-card-actions>
          </v-card>

          <CombinedTimetable
            v-else-if="calendar"
            :calendar="calendar"
            :dirty="dirty"
          />
        </template>

        <template v-if="activeTab === 'members'">
          <div
            v-if="calendarLoading"
            class="d-flex flex-column align-center py-8"
            role="status"
            aria-live="polite"
          >
            <v-progress-circular indeterminate color="primary" size="28" />
            <p class="text-body-2 text-medium-emphasis mt-3 mb-0">
              {{ ru.planner.loadingCalendar }}
            </p>
          </div>

          <v-card v-else-if="calendarError" variant="tonal" class="state-card">
            <v-card-text>
              <v-alert type="error" variant="tonal" density="comfortable" role="alert">
                {{ calendarError }}
              </v-alert>
            </v-card-text>
            <v-card-actions class="px-4 pb-4">
              <v-spacer />
              <v-btn color="primary" variant="flat" prepend-icon="mdi-refresh" @click="retryCalendar">
                {{ ru.planner.retry }}
              </v-btn>
            </v-card-actions>
          </v-card>

          <template v-else-if="calendar">
            <template v-if="!selectedMember">
              <v-card variant="tonal" class="state-card">
                <v-card-item>
                  <v-card-title class="text-h6">
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
                      <v-icon>mdi-chevron-right</v-icon>
                    </template>
                  </v-list-item>
                </v-list>
              </v-card>
            </template>

            <template v-else>
              <div
                v-if="memberLoading"
                class="d-flex flex-column align-center py-8"
                role="status"
                aria-live="polite"
              >
                <v-progress-circular indeterminate color="primary" size="28" />
                <p class="text-body-2 text-medium-emphasis mt-3 mb-0">
                  {{ ru.planner.memberScheduleLoading }}
                </p>
              </div>

              <v-card v-else-if="memberError" variant="tonal" class="state-card">
                <v-card-text>
                  <v-alert type="error" variant="tonal" density="comfortable" role="alert">
                    {{ memberError }}
                  </v-alert>
                </v-card-text>
                <v-card-actions class="px-4 pb-4">
                  <v-btn variant="text" @click="backToMembers">
                    {{ ru.planner.backToMembers }}
                  </v-btn>
                  <v-spacer />
                  <v-btn color="primary" variant="flat" prepend-icon="mdi-refresh" @click="retryMember">
                    {{ ru.planner.retry }}
                  </v-btn>
                </v-card-actions>
              </v-card>

              <MemberSchedule
                v-else-if="memberCalendar"
                :response="memberCalendar"
                @back="backToMembers"
              />
            </template>
          </template>
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
  border: 1px solid rgba(255, 255, 255, 0.08);
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
