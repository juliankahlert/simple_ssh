import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import HeroSection from './HeroSection.vue'

describe('HeroSection', () => {
  it('renders heading with accent', () => {
    const wrapper = mount(HeroSection)

    const title = wrapper.find('h1')
    expect(title.exists()).toBe(true)
    expect(title.text()).toContain('SSH made')
    expect(title.find('.accent').exists()).toBe(true)
  })

  it('renders badge with pulsing dot', () => {
    const wrapper = mount(HeroSection)

    const badge = wrapper.find('.hero-badge')
    expect(badge.exists()).toBe(true)
    expect(badge.text()).toContain('Async SSH library')
    expect(badge.find('.dot').exists()).toBe(true)
  })

  it('renders subtitle with library references', () => {
    const wrapper = mount(HeroSection)

    const subtitle = wrapper.find('.hero-sub')
    expect(subtitle.exists()).toBe(true)
    expect(subtitle.text()).toContain('tokio')
    expect(subtitle.text()).toContain('russh')
  })

  it('renders CTA links', () => {
    const wrapper = mount(HeroSection)

    const links = wrapper.findAll('.hero-actions a')
    const texts = links.map((b) => b.text())
    expect(texts).toContain('Get Started')
    expect(texts).toContain('View Examples')
  })
})
