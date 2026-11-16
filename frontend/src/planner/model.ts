import { ru } from '../i18n/ru'
import type { PreferenceInterval } from '../types/api'

export const SLOTS_PER_DAY = 48
export const SLOT_MINUTES = 30
export const WEEK_SLOTS = 7 * SLOTS_PER_DAY

export interface DayInfo {
  value: number
  short: string
  full: string
}

export const DAYS: readonly DayInfo[] = [1, 2, 3, 4, 5, 6, 7].map(
  (value, index) => ({
    value,
    short: ru.planner.dayShort[index] ?? '',
    full: ru.planner.dayFull[index] ?? '',
  }),
)

export function slotIndex(dayIndex: number, slot: number): number {
  return dayIndex * SLOTS_PER_DAY + slot
}

/** Expands stored half-open intervals into selected 30-minute slot indices. */
export function intervalsToSlots(
  intervals: readonly PreferenceInterval[],
): number[] {
  const slots = new Set<number>()

  for (const interval of intervals) {
    if (
      !Number.isInteger(interval.weekday) ||
      interval.weekday < 1 ||
      interval.weekday > 7 ||
      !Number.isInteger(interval.start_minute) ||
      !Number.isInteger(interval.end_minute) ||
      interval.start_minute < 0 ||
      interval.end_minute > 1440 ||
      interval.start_minute >= interval.end_minute
    ) {
      continue
    }

    const dayIndex = interval.weekday - 1
    const firstSlot = Math.floor(interval.start_minute / SLOT_MINUTES)
    const lastSlot = Math.ceil(interval.end_minute / SLOT_MINUTES)
    for (let slot = firstSlot; slot < lastSlot; slot += 1) {
      slots.add(slotIndex(dayIndex, slot))
    }
  }

  return [...slots].sort((left, right) => left - right)
}

/** Merges adjacent selected slots into half-open weekday intervals. */
export function slotsToIntervals(slots: readonly number[]): PreferenceInterval[] {
  const selected = new Set(
    slots.filter((slot) => Number.isInteger(slot) && slot >= 0 && slot < WEEK_SLOTS),
  )
  const intervals: PreferenceInterval[] = []

  for (let dayIndex = 0; dayIndex < 7; dayIndex += 1) {
    let runStart: number | null = null

    for (let slot = 0; slot < SLOTS_PER_DAY; slot += 1) {
      const isSelected = selected.has(slotIndex(dayIndex, slot))

      if (isSelected && runStart === null) {
        runStart = slot
      } else if (!isSelected && runStart !== null) {
        intervals.push({
          weekday: dayIndex + 1,
          start_minute: runStart * SLOT_MINUTES,
          end_minute: slot * SLOT_MINUTES,
        })
        runStart = null
      }
    }

    if (runStart !== null) {
      intervals.push({
        weekday: dayIndex + 1,
        start_minute: runStart * SLOT_MINUTES,
        end_minute: 1440,
      })
    }
  }

  return intervals
}

export function sameSlots(
  left: readonly number[],
  right: readonly number[],
): boolean {
  if (left.length !== right.length) {
    return false
  }
  const a = [...left].sort((x, y) => x - y)
  const b = [...right].sort((x, y) => x - y)
  return a.every((value, index) => value === b[index])
}

export function formatTime(minutes: number): string {
  const hours = Math.floor(minutes / 60)
  const mins = minutes % 60
  return `${String(hours).padStart(2, '0')}:${String(mins).padStart(2, '0')}`
}
