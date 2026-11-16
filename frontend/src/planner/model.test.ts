import { describe, expect, it } from 'vitest'

import type { PreferenceInterval } from '../types/api'
import {
  SLOTS_PER_DAY,
  formatTime,
  intervalsToSlots,
  sameSlots,
  slotIndex,
  slotsToIntervals,
} from './model'

function interval(
  weekday: number,
  startMinute: number,
  endMinute: number,
): PreferenceInterval {
  return {
    weekday,
    start_minute: startMinute,
    end_minute: endMinute,
  }
}

describe('planner grid model', () => {
  it('expands an interval into half-hour slots', () => {
    expect(intervalsToSlots([interval(1, 18 * 60, 20 * 60)])).toEqual([
      36, 37, 38, 39,
    ])
  })

  it('maps the first Monday slot and the full Sunday', () => {
    expect(slotIndex(0, 0)).toBe(0)
    expect(intervalsToSlots([interval(1, 0, 30)])).toEqual([0])

    const sunday = intervalsToSlots([interval(7, 0, 1440)])
    expect(sunday).toHaveLength(SLOTS_PER_DAY)
    expect(sunday[0]).toBe(6 * SLOTS_PER_DAY)
    expect(sunday.at(-1)).toBe(7 * SLOTS_PER_DAY - 1)
  })

  it('merges adjacent selected slots per day only', () => {
    const slots = [0, 1, 2, 48, 49]
    expect(slotsToIntervals(slots)).toEqual([
      interval(1, 0, 90),
      interval(2, 0, 60),
    ])
  })

  it('round-trips intervals and slots', () => {
    const intervals = [
      interval(1, 600, 660),
      interval(3, 660, 720),
      interval(7, 1380, 1440),
    ]
    expect(slotsToIntervals(intervalsToSlots(intervals))).toEqual(intervals)
  })

  it('ignores malformed intervals instead of crashing', () => {
    expect(
      intervalsToSlots([
        interval(0, 0, 60),
        interval(8, 0, 60),
        interval(1, 60, 0),
        interval(1, 0, 1500),
      ]),
    ).toEqual([])
  })

  it('compares selections regardless of order', () => {
    expect(sameSlots([1, 2, 3], [3, 1, 2])).toBe(true)
    expect(sameSlots([1, 2], [1, 2, 3])).toBe(false)
    expect(sameSlots([], [])).toBe(true)
  })

  it('formats half-hour times including midnight end', () => {
    expect(formatTime(0)).toBe('00:00')
    expect(formatTime(570)).toBe('09:30')
    expect(formatTime(1440)).toBe('24:00')
  })
})
