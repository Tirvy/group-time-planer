import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'

import WeekGrid from './WeekGrid.vue'

function mountGrid(modelValue: number[] = []) {
  return mount(WeekGrid, {
    props: { modelValue, color: '#4DD0E1' },
    attachTo: document.body,
  })
}

function cells(wrapper: ReturnType<typeof mountGrid>) {
  return wrapper.findAll('button.slot')
}

function lastEmitted(wrapper: ReturnType<typeof mountGrid>): number[] {
  const events = wrapper.emitted('update:modelValue')
  const last = events?.at(-1)
  return (last?.[0] as number[] | undefined) ?? []
}

describe('WeekGrid interaction', () => {
  it('renders one accessible cell per half hour of the week', () => {
    const wrapper = mountGrid()
    const all = cells(wrapper)

    expect(all).toHaveLength(7 * 48)
    expect(all[0]?.attributes('aria-pressed')).toBe('false')
    expect(all[0]?.attributes('aria-label')).toContain('00:00')

    wrapper.unmount()
  })

  it('toggles a cell on tap', async () => {
    const wrapper = mountGrid()
    const first = cells(wrapper)[0]!

    await first.trigger('pointerdown', { button: 0, clientX: 10, clientY: 10 })
    window.dispatchEvent(new Event('pointerup'))

    expect(lastEmitted(wrapper)).toEqual([0])
    wrapper.unmount()
  })

  it('supports keyboard activation through the click event', async () => {
    const wrapper = mountGrid([0])
    const first = cells(wrapper)[0]!

    await first.trigger('click', { detail: 0 })

    expect(lastEmitted(wrapper)).toEqual([])
    wrapper.unmount()
  })

  it('paints a horizontal drag across cells without toggling twice', async () => {
    const wrapper = mountGrid()
    const all = cells(wrapper)

    const elementFromPoint = vi.fn(() => all[1]!.element)
    Object.defineProperty(document, 'elementFromPoint', {
      configurable: true,
      value: elementFromPoint,
    })

    await all[0]!.trigger('pointerdown', {
      button: 0,
      clientX: 0,
      clientY: 0,
      pointerType: 'mouse',
    })
    await wrapper
      .find('.week-grid')
      .trigger('pointermove', { clientX: 20, clientY: 0, pointerType: 'mouse' })
    await wrapper
      .find('.week-grid')
      .trigger('pointermove', { clientX: 40, clientY: 0, pointerType: 'mouse' })
    window.dispatchEvent(new Event('pointerup'))

    // Cell order is slot-major, so the second DOM cell is Tuesday 00:00.
    expect(lastEmitted(wrapper)).toEqual([0, 48])
    wrapper.unmount()
  })

  it('keeps vertical touch gestures as page scrolling', async () => {
    const wrapper = mountGrid()

    await cells(wrapper)[0]!.trigger('pointerdown', {
      button: 0,
      clientX: 0,
      clientY: 0,
      pointerType: 'touch',
    })
    await wrapper
      .find('.week-grid')
      .trigger('pointermove', { clientX: 1, clientY: 30, pointerType: 'touch' })
    window.dispatchEvent(new Event('pointerup'))

    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    wrapper.unmount()
  })
})
