import type { CalendarMember, CalendarSegment } from '../types/api'

export const READ_SLOT_MINUTES = 15
export const READ_SLOTS_PER_DAY = 96

/** Combined grid: [day][15-minute slot] -> members preferring that slice. */
export function buildCombinedGrid(
  members: readonly CalendarMember[],
): CalendarMember[][][] {
  const grid: CalendarMember[][][] = Array.from({ length: 7 }, () =>
    Array.from({ length: READ_SLOTS_PER_DAY }, () => [] as CalendarMember[]),
  )

  for (const member of members) {
    for (const segment of member.segments) {
      if (
        !Number.isInteger(segment.day) ||
        segment.day < 0 ||
        segment.day > 6 ||
        segment.start_minute < 0 ||
        segment.end_minute > 1440 ||
        segment.start_minute >= segment.end_minute
      ) {
        continue
      }

      const first = Math.max(
        0,
        Math.floor(segment.start_minute / READ_SLOT_MINUTES),
      )
      const last = Math.min(
        READ_SLOTS_PER_DAY,
        Math.ceil(segment.end_minute / READ_SLOT_MINUTES),
      )

      for (let slot = first; slot < last; slot += 1) {
        const cell = grid[segment.day]?.[slot]
        if (cell && !cell.some((candidate) => candidate.user_id === member.user_id)) {
          cell.push(member)
        }
      }
    }
  }

  return grid
}

export function segmentsForDay(
  member: CalendarMember,
  day: number,
): CalendarSegment[] {
  return member.segments
    .filter((segment) => segment.day === day)
    .sort(
      (left, right) =>
        left.start_minute - right.start_minute ||
        left.end_minute - right.end_minute,
    )
}
