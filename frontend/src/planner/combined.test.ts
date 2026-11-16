import { describe, expect, it } from 'vitest'

import type { CalendarMember, CalendarSegment } from '../types/api'
import { buildCombinedGrid, segmentsForDay } from './combined'

function segment(
  day: number,
  startMinute: number,
  endMinute: number,
): CalendarSegment {
  return {
    day,
    start_minute: startMinute,
    end_minute: endMinute,
    start: new Date(Date.UTC(2026, 10, 16 + day, 0, startMinute)).toISOString(),
    end: new Date(Date.UTC(2026, 10, 16 + day, 0, endMinute)).toISOString(),
  }
}

function member(
  id: string,
  color: string,
  segments: CalendarSegment[],
): CalendarMember {
  return {
    user_id: id,
    first_name: 'Test',
    last_name: null,
    username: null,
    role: 'member',
    color,
    setup_state: 'completed',
    segments,
  }
}

describe('combined timetable model', () => {
  it('maps fractional viewer times into 15-minute slices', () => {
    const a = member('a', '#111111', [segment(0, 22 * 60 + 45, 23 * 60 + 45)])
    const grid = buildCombinedGrid([a])

    // 22:45 -> slot 91, 23:45 -> slot 95.
    expect(grid[0]?.[90]).toEqual([])
    expect(grid[0]?.[91]?.[0]?.user_id).toBe('a')
    expect(grid[0]?.[94]?.[0]?.user_id).toBe('a')
    expect(grid[0]?.[95]).toEqual([])
  })

  it('keeps overlapping members in the same slice in order', () => {
    const a = member('a', '#111111', [segment(0, 18 * 60, 19 * 60)])
    const b = member('b', '#222222', [segment(0, 18 * 60 + 30, 19 * 60 + 30)])
    const c = member('c', '#333333', [])

    const grid = buildCombinedGrid([a, b, c])
    expect(grid[0]?.[72]?.map((entry) => entry.user_id)).toEqual(['a'])
    expect(grid[0]?.[74]?.map((entry) => entry.user_id)).toEqual(['a', 'b'])
    expect(grid[0]?.[77]?.map((entry) => entry.user_id)).toEqual(['b'])
    expect(grid[0]?.[78]).toEqual([])
  })

  it('ignores malformed segments', () => {
    const a = member('a', '#111111', [
      segment(0, -15, 30),
      segment(0, 30, 30),
      segment(9, 0, 30),
    ])
    expect(buildCombinedGrid([a]).flat(2)).toEqual([])
  })

  it('sorts a member day schedule', () => {
    const a = member('a', '#111111', [
      segment(1, 1200, 1260),
      segment(1, 600, 660),
    ])
    expect(segmentsForDay(a, 1).map((item) => item.start_minute)).toEqual([
      600, 1200,
    ])
  })
})
