import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import InstallSection from './InstallSection.vue'

const originalClipboard = Object.getOwnPropertyDescriptor(navigator, 'clipboard');

beforeEach(() => {
  Object.defineProperty(navigator, 'clipboard', {
    value: {
      writeText: vi.fn()
    },
    writable: true,
    configurable: true
  })
})

afterEach(() => {
  vi.restoreAllMocks();
  if (originalClipboard) {
    Object.defineProperty(navigator, 'clipboard', originalClipboard);
  } else {
    delete (navigator as any).clipboard;
  }
})

describe('InstallSection', () => {
  it('renders card container', () => {
    const wrapper = mount(InstallSection)
    const card = wrapper.find('.install-card')
    expect(card.exists()).toBe(true)
  })

  it('renders section header', () => {
    const wrapper = mount(InstallSection)
    const header = wrapper.find('.section-header')
    expect(header.exists()).toBe(true)
    expect(header.text()).toContain('Install')
  })

  it('renders both tabs (Library and CLI Tool)', () => {
    const wrapper = mount(InstallSection)
    const tabs = wrapper.findAll('.install-tab')
    expect(tabs.length).toBe(2)
    expect(tabs[0].text()).toBe('Library')
    expect(tabs[1].text()).toBe('CLI Tool')
  })

  it('shows Library snippet when Library tab is active by default', () => {
    const wrapper = mount(InstallSection)
    const activeTab = wrapper.find('.install-tab.active')
    expect(activeTab.exists()).toBe(true)
    expect(activeTab.text()).toBe('Library')
    const snippet = wrapper.find('.install-snippet')
    expect(snippet.exists()).toBe(true)
    expect(snippet.text()).toContain('cargo add simple_ssh')
  })

  it('shows CLI Tool snippet when CLI Tool tab is active', async () => {
    const wrapper = mount(InstallSection)
    const cliTab = wrapper.findAll('.install-tab')[1]

    await cliTab.trigger('click')
    const activeTab = wrapper.find('.install-tab.active')
    expect(activeTab.text()).toBe('CLI Tool')

    const snippet = wrapper.find('.install-snippet')
    expect(snippet.text()).toContain('cargo install simple_ssh --features cli')
  })

  it('toggles between tabs correctly', async () => {
    const wrapper = mount(InstallSection)
    const tabs = wrapper.findAll('.install-tab')

    expect(tabs[0].classes()).toContain('active')

    await tabs[1].trigger('click')
    expect(tabs[0].classes()).not.toContain('active')
    expect(tabs[1].classes()).toContain('active')

    await tabs[0].trigger('click')
    expect(tabs[0].classes()).toContain('active')
    expect(tabs[1].classes()).not.toContain('active')
  })

  it('copies snippet to clipboard when clicking it', async () => {
    const wrapper = mount(InstallSection)
    const copyBtn = wrapper.find('.copy-btn')

    await copyBtn.trigger('click')
    await wrapper.vm.$nextTick()
    expect((navigator.clipboard as any).writeText).toHaveBeenCalledWith('cargo add simple_ssh')
  })

  it('changes border color when snippet is copied', async () => {
    const wrapper = mount(InstallSection)
    const snippet = wrapper.find('.install-snippet')

    expect(snippet.classes()).not.toContain('copied')

    await wrapper.find('.copy-btn').trigger('click')
    await wrapper.vm.$nextTick()
    expect(snippet.classes()).toContain('copied')
  })

  it('has three external links at bottom', () => {
    const wrapper = mount(InstallSection)
    const links = wrapper.findAll('.install-links a')

    expect(links.length).toBe(3)
    expect(links[0].text()).toContain('GitHub')
    expect(links[1].text()).toContain('Documentation')
    expect(links[2].text()).toContain('Crates.io')
  })

  it('each link has appropriate icon', () => {
    const wrapper = mount(InstallSection)
    const links = wrapper.findAll('.install-links a')

    links.forEach(link => {
      const icon = link.find('svg')
      expect(icon.exists()).toBe(true)
    })
  })

  it('has clickable tabs', () => {
    const wrapper = mount(InstallSection)

    const tabs = wrapper.findAll('.install-tab')
    expect(tabs.length).toBe(2)
    expect(tabs[0].isVisible()).toBe(true)
    expect(tabs[1].isVisible()).toBe(true)
  })
})
