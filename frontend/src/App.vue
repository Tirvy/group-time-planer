<script setup lang="ts">
import { onMounted, ref } from 'vue'

import { mdiClockEditOutline, mdiRefresh, mdiShieldAlertOutline } from '@mdi/js'

import TimezoneSetup from './components/TimezoneSetup.vue'
import GroupsView from './components/GroupsView.vue'
import { useSession } from './composables/useSession'
import { watchTelegramTheme } from './telegram/theme'
import { telegramIcon } from './telegram/icon'
import { ru } from './i18n/ru'

const {
  state,
  user,
  initData,
  errorMessage,
  sessionExpired,
  timezoneRequired,
  saving,
  saveError,
  start,
  saveTimezone,
  clearSaveError,
} = useSession()

const editOpen = ref(false)
const savedSnackbar = ref(false)

function openTimezoneEditor(): void {
  clearSaveError()
  editOpen.value = true
}

function closeTimezoneEditor(): void {
  clearSaveError()
  editOpen.value = false
}

onMounted(() => {
  watchTelegramTheme()
  void start()
})

async function saveInitialTimezone(timezone: string): Promise<void> {
  if (await saveTimezone(timezone)) {
    savedSnackbar.value = true
  }
}

async function saveChangedTimezone(timezone: string): Promise<void> {
  if (await saveTimezone(timezone)) {
    savedSnackbar.value = true
    editOpen.value = false
  }
}</script>

<template>
  <v-app>
    <v-main>
      <v-container class="app-shell" max-width="560">
        <div
          v-if="state === 'loading'"
          class="d-flex flex-column align-center justify-center py-12"
          role="status"
          aria-live="polite"
        >
          <v-progress-circular indeterminate color="primary" size="36" />
          <p class="text-body-medium text-medium-emphasis mt-4 mb-0">
            {{ ru.auth.checking }}
          </p>
        </div>

        <v-card v-else-if="state === 'no-telegram'" variant="tonal" class="state-card">
          <v-card-item>
            <template #prepend>
              <v-icon color="primary" size="32" :icon="telegramIcon" />
            </template>
            <v-card-title>{{ ru.auth.noTelegramTitle }}</v-card-title>
            <v-card-subtitle class="text-wrap">
              {{ ru.auth.noTelegramBody }}
            </v-card-subtitle>
          </v-card-item>
        </v-card>

        <v-card v-else-if="state === 'error'" variant="tonal" class="state-card">
          <v-card-item>
            <template #prepend>
              <v-icon color="error" size="32" :icon="mdiShieldAlertOutline" />
            </template>
            <v-card-title>{{ ru.auth.errorTitle }}</v-card-title>
          </v-card-item>
          <v-card-text>
            <v-alert type="error" variant="tonal" density="comfortable" role="alert">
              {{ errorMessage }}
            </v-alert>
            <p v-if="sessionExpired" class="text-body-small text-medium-emphasis mt-3 mb-0">
              {{ ru.auth.expiredHint }}
            </p>
          </v-card-text>
          <v-card-actions class="px-4 pb-4">
            <v-spacer />
            <v-btn color="primary" variant="flat" :prepend-icon="mdiRefresh" @click="start">
              {{ ru.auth.retry }}
            </v-btn>
          </v-card-actions>
        </v-card>

        <TimezoneSetup
          v-else-if="timezoneRequired"
          :current="null"
          :saving="saving"
          :error="saveError"
          @save="saveInitialTimezone"
        />

        <template v-else-if="state === 'ready' && user">
          <GroupsView :init-data="initData" />

          <v-card variant="tonal" class="state-card mt-4">
            <div class="timezone-row px-4 py-2">
              <span class="timezone-name text-body-small text-medium-emphasis">
                {{ user.timezone }}
              </span>
              <v-btn
                class="timezone-action text-body-small"
                size="small"
                variant="text"
                color="primary"
                :prepend-icon="mdiClockEditOutline"
                @click="openTimezoneEditor"
              >
                {{ ru.home.changeTimezone }}
              </v-btn>
            </div>
          </v-card>
        </template>
      </v-container>
    </v-main>

    <v-dialog v-model="editOpen" max-width="560" scrollable>
      <TimezoneSetup
        :current="user?.timezone ?? null"
        :saving="saving"
        :error="saveError"
        @save="saveChangedTimezone"
        @cancel="closeTimezoneEditor"
      />
    </v-dialog>

    <v-snackbar v-model="savedSnackbar" color="success" :timeout="2500">
      {{ ru.timezone.saved }}
    </v-snackbar>
  </v-app>
</template>

<style scoped>
.app-shell {
  padding-top: calc(12px + env(safe-area-inset-top, 0px));
  padding-bottom: calc(72px + env(safe-area-inset-bottom, 0px));
}

.timezone-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.timezone-name {
  flex: 1 1 auto;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.timezone-action {
  flex: 0 0 auto;
  white-space: nowrap;
  font-size: 0.75rem;
  line-height: 1rem;
}

.state-card {
  border: 1px solid var(--app-border);
}
</style>
