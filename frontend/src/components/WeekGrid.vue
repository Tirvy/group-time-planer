<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { DAYS, SLOTS_PER_DAY, formatTime, slotIndex } from '../planner/model'

const props = defineProps<{
  modelValue: number[]
  color: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: number[]]
}>()

interface DragStart {
  day: number
  slot: number
  x: number
  y: number
  value: boolean
  pointerType: string
  moved: boolean
}

const draft = ref<Set<number>>(new Set(props.modelValue))
const drag = ref<DragStart | null>(null)
const lastPainted = ref<number | null>(null)

watch(
  () => props.modelValue,
  (value) => {
    if (!drag.value) {
      draft.value = new Set(value)
    }
  },
  { deep: true },
)

function isSelected(dayIndex: number, slot: number): boolean {
  return draft.value.has(slotIndex(dayIndex, slot))
}

function commitDraft(next: Set<number>): void {
  draft.value = next
  emit(
    'update:modelValue',
    [...next].sort((left, right) => left - right),
  )
}

function paint(dayIndex: number, slot: number, value: boolean): void {
  const index = slotIndex(dayIndex, slot)
  if (lastPainted.value === index) return
  lastPainted.value = index

  const next = new Set(draft.value)
  if (value) {
    next.add(index)
  } else {
    next.delete(index)
  }
  commitDraft(next)
}

function onCellPointerDown(
  dayIndex: number,
  slot: number,
  event: PointerEvent,
): void {
  if (event.button !== 0) return

  drag.value = {
    day: dayIndex,
    slot,
    x: event.clientX,
    y: event.clientY,
    value: !isSelected(dayIndex, slot),
    pointerType: event.pointerType,
    moved: false,
  }
  lastPainted.value = null
}

function onGridPointerMove(event: PointerEvent): void {
  const current = drag.value
  if (!current) return

  const dx = event.clientX - current.x
  const dy = event.clientY - current.y

  if (!current.moved) {
    if (Math.hypot(dx, dy) < 6) return

    // Touch gestures dominated by vertical movement remain page scrolling.
    if (current.pointerType !== 'mouse' && Math.abs(dy) > Math.abs(dx)) {
      drag.value = null
      return
    }

    current.moved = true
    paint(current.day, current.slot, current.value)
  }

  event.preventDefault()

  const element = document.elementFromPoint(event.clientX, event.clientY)
  const cell = element?.closest<HTMLElement>('[data-day][data-slot]')
  if (!cell) return

  const dayIndex = Number(cell.dataset.day)
  const slot = Number(cell.dataset.slot)
  if (Number.isInteger(dayIndex) && Number.isInteger(slot)) {
    paint(dayIndex, slot, current.value)
  }
}

function finishDrag(commitTap: boolean): void {
  const current = drag.value
  if (!current) return

  drag.value = null
  lastPainted.value = null

  if (commitTap && !current.moved) {
    const next = new Set(draft.value)
    const index = slotIndex(current.day, current.slot)
    if (current.value) {
      next.add(index)
    } else {
      next.delete(index)
    }
    commitDraft(next)
  }
}

function onPointerUp(): void {
  finishDrag(true)
}

function onPointerCancel(): void {
  finishDrag(false)
}

function onCellClick(
  event: MouseEvent,
  dayIndex: number,
  slot: number,
): void {
  // Pointer taps are committed on pointerup; detail === 0 means keyboard.
  if (event.detail !== 0) return

  const next = new Set(draft.value)
  const index = slotIndex(dayIndex, slot)
  if (next.has(index)) {
    next.delete(index)
  } else {
    next.add(index)
  }
  commitDraft(next)
}

onMounted(() => {
  window.addEventListener('pointerup', onPointerUp)
  window.addEventListener('pointercancel', onPointerCancel)
})

onBeforeUnmount(() => {
  window.removeEventListener('pointerup', onPointerUp)
  window.removeEventListener('pointercancel', onPointerCancel)
})
</script>

<template>
  <div class="week-grid" @pointermove="onGridPointerMove">
    <div class="calendar-grid" :style="{ '--member-color': props.color }">
      <div class="corner" aria-hidden="true" />
      <div
        v-for="day in DAYS"
        :key="day.value"
        class="day-head"
        :title="day.full"
      >
        {{ day.short }}
      </div>

      <template v-for="(_, slot) in SLOTS_PER_DAY" :key="slot">
        <div
          class="time-label"
          :class="slot % 2 === 0 ? 'time-label--hour' : 'time-label--half'"
        >
          {{ formatTime(slot * 30) }}
        </div>
        <button
          v-for="(day, dayIndex) in DAYS"
          :key="day.value"
          type="button"
          class="slot"
          :class="{
            'slot--selected': isSelected(dayIndex, slot),
            'slot--hour': slot % 2 === 0,
          }"
          :data-day="dayIndex"
          :data-slot="slot"
          :aria-label="`${day.full}, ${formatTime(slot * 30)}`"
          :aria-pressed="isSelected(dayIndex, slot)"
          @pointerdown="onCellPointerDown(dayIndex, slot, $event)"
          @click="onCellClick($event, dayIndex, slot)"
        />
      </template>

      <div class="corner corner--bottom" aria-hidden="true" />
      <div
        v-for="day in DAYS"
        :key="`bottom-${day.value}`"
        class="day-head day-head--bottom"
        :title="day.full"
        aria-hidden="true"
      >
        {{ day.short }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.week-grid {
  width: 100%;
  overflow-x: auto;
  touch-action: pan-y;
  user-select: none;
  -webkit-user-select: none;
  -webkit-touch-callout: none;
}

.calendar-grid {
  display: grid;
  grid-template-columns: 44px repeat(7, minmax(34px, 1fr));
  min-width: 282px;
}

.corner {
  height: 30px;
}

.day-head {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 30px;
  font-size: 0.75rem;
  font-weight: 600;
  color: rgba(var(--v-theme-on-surface), 0.85);
  border-bottom: 1px solid rgba(255, 255, 255, 0.12);
}

.day-head--bottom {
  border-bottom: 0;
  border-top: 1px solid rgba(255, 255, 255, 0.12);
}

.corner--bottom {
  border-top: 1px solid rgba(255, 255, 255, 0.12);
}

.time-label {
  height: 28px;
  font-size: 0.6rem;
  line-height: 28px;
  text-align: right;
  padding-right: 5px;
  color: rgba(var(--v-theme-on-surface), 0.5);
  white-space: nowrap;
}

.time-label--hour {
  border-top: 1px solid rgba(255, 255, 255, 0.12);
}

.time-label--half {
  color: rgba(var(--v-theme-on-surface), 0.3);
}

.slot {
  height: 28px;
  padding: 0;
  border: 0;
  border-top: 1px solid rgba(255, 255, 255, 0.04);
  border-left: 1px solid rgba(255, 255, 255, 0.04);
  background: rgba(255, 255, 255, 0.025);
  cursor: pointer;
  touch-action: pan-y;
  -webkit-tap-highlight-color: transparent;
}

.slot--hour {
  border-top-color: rgba(255, 255, 255, 0.12);
}

.slot--selected {
  background: var(--member-color);
  border-top-color: rgba(0, 0, 0, 0.15);
  border-left-color: rgba(0, 0, 0, 0.15);
}

.slot:focus-visible {
  position: relative;
  z-index: 1;
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: -2px;
}
</style>
