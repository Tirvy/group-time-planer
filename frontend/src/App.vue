<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'

import TimezoneSetup from './components/TimezoneSetup.vue'
import GroupsView from './components/GroupsView.vue'
import { useSession } from './composables/useSession'
import { ru } from './i18n/ru'

const {
  state,
  user,
  initData,
  errorCode,
  errorMessage,
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

const showReopenHint = computed(
  () => errorCode.value === 'AUTH_EXPIRED' || errorCode.value === 'AUTH_INVALID',
)

onMounted(start)

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
              <v-icon color="primary" size="32">mdi-telegram</v-icon>
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
              <v-icon color="error" size="32">mdi-shield-alert-outline</v-icon>
            </template>
            <v-card-title>{{ ru.auth.errorTitle }}</v-card-title>
          </v-card-item>
          <v-card-text>
            <v-alert type="error" variant="tonal" density="comfortable" role="alert">
              {{ errorMessage }}
            </v-alert>
            <p v-if="showReopenHint" class="text-body-small text-medium-emphasis mt-3 mb-0">
              {{ ru.auth.expiredHint }}
            </p>
          </v-card-text>
          <v-card-actions class="px-4 pb-4">
            <v-spacer />
            <v-btn color="primary" variant="flat" prepend-icon="mdi-refresh" @click="start">
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
          <v-card variant="tonal" class="state-card mb-4">
            <div class="d-flex align-center flex-wrap ga-2 px-4 py-2">
              <span class="text-body-small text-medium-emphasis">
                {{ user.timezone }}
              </span>
              <v-spacer />
              <v-btn
                size="small"
                variant="text"
                color="primary"
                prepend-icon="mdi-clock-edit-outline"
                @click="openTimezoneEditor"
              >
                {{ ru.home.changeTimezone }}
              </v-btn>
            </div>
          </v-card>

          <GroupsView :init-data="initData" />
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
  padding-bottom: calc(24px + env(safe-area-inset-bottom, 0px));
}

.state-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
}
</style>
