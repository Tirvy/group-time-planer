import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

import BestTimeView from './BestTimeView.vue'
import type { BestTimeResponse } from '../types/api'

const vuetify = createVuetify({ components, directives })

function bestTime(): BestTimeResponse {
  return {
    group: { id: '1', name: 'Рейд' },
    viewer_timezone: 'Europe/Berlin',
    reference_week: {
      start_date: '2026-11-16',
      end_date: '2026-11-22',
      start: '2026-11-16T00:00:00+01:00',
      end: '2026-11-23T00:00:00+01:00',
    },
    total_member_count: 2,
    configured_member_count: 2,
    pending_member_count: 0,
    members: [
      { user_id: '1', first_name: 'Иван', last_name: null, username: null, color: '#4DD0E1' },
      { user_id: '2', first_name: 'Пётр', last_name: null, username: null, color: '#7E57C2' },
    ],
    intervals: [
      {
        day: 0,
        start_minute: 19 * 60,
        end_minute: 20 * 60,
        start: '2026-11-16T19:00:00+01:00',
        end: '2026-11-16T20:00:00+01:00',
        count: 2,
        members: ['1', '2'],
      },
      {
        day: 1,
        start_minute: 18 * 60,
        end_minute: 19 * 60,
        start: '2026-11-17T18:00:00+01:00',
        end: '2026-11-17T19:00:00+01:00',
        count: 1,
        members: ['1'],
      },
    ],
  }
}

function mountView() {
  return mount(BestTimeView, {
    props: { bestTime: bestTime(), dirty: false },
    global: { plugins: [vuetify] },
    attachTo: document.body,
  })
}

describe('BestTimeView', () => {
  it('shows only ranges with at least two available members by default', () => {
    const wrapper = mountView()
    const rows = wrapper.findAll('button.rank-row')

    expect(rows).toHaveLength(1)
    expect(rows[0]?.text()).toContain('доступны: 2 участника')
    expect(rows[0]?.text()).toContain('19:00–20:00')

    wrapper.unmount()
  })

  it('reveals single-member ranges through the filter', async () => {
    const wrapper = mountView()

    const toggleButtons = wrapper.findAll('.v-btn-toggle button')
    expect(toggleButtons).toHaveLength(2)
    await toggleButtons[1]!.trigger('click')

    const rows = wrapper.findAll('button.rank-row')
    expect(rows).toHaveLength(2)
    expect(rows[1]?.text()).toContain('доступен 1 участник')

    wrapper.unmount()
  })

  it('explains when no range has overlapping availability', () => {
    const singleOnly = bestTime()
    singleOnly.intervals = singleOnly.intervals.slice(1)
    const wrapper = mount(BestTimeView, {
      props: { bestTime: singleOnly, dirty: false },
      global: { plugins: [vuetify] },
    })

    expect(wrapper.findAll('button.rank-row')).toHaveLength(0)
    expect(wrapper.text()).toContain('Нет времени, когда доступны двое или больше.')

    wrapper.unmount()
  })
})
