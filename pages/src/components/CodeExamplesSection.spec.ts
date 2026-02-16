import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import CodeExamplesSection from './CodeExamplesSection.vue'

class MockIntersectionObserver {
  constructor(_callback: IntersectionObserverCallback) {}
  observe() {}
  unobserve() {}
  disconnect() {}
  takeRecords() {
    return [];
  }
}

let originalIntersectionObserver: typeof IntersectionObserver | undefined;
let originalClipboard: Clipboard | undefined;

beforeEach(() => {
  originalIntersectionObserver = global.IntersectionObserver;
  global.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

  originalClipboard = navigator.clipboard;
  Object.defineProperty(navigator, 'clipboard', {
    value: {
      writeText: vi.fn()
    },
    writable: true,
    configurable: true
  })
})

afterEach(() => {
  global.IntersectionObserver = originalIntersectionObserver ?? global.IntersectionObserver;
  Object.defineProperty(navigator, 'clipboard', {
    value: originalClipboard,
    writable: true,
    configurable: true
  });
  vi.restoreAllMocks();
})

describe('CodeExamplesSection', () => {
  it('renders section label', () => {
    const wrapper = mount(CodeExamplesSection)
    const label = wrapper.find('.section-label')
    expect(label.exists()).toBe(true)
    expect(label.text()).toBe('Examples')
  })

  it('renders section title', () => {
    const wrapper = mount(CodeExamplesSection)
    const title = wrapper.find('.section-title')
    expect(title.exists()).toBe(true)
    expect(title.text()).toContain('Copy, paste, run.')
  })

  it('renders section description', () => {
    const wrapper = mount(CodeExamplesSection)
    const description = wrapper.find('.section-desc')
    expect(description.exists()).toBe(true)
    expect(description.text()).toContain('Practical examples for common SSH operations.')
  })

  it('renders Cargo.toml code block with correct content', () => {
    const wrapper = mount(CodeExamplesSection)
    const codeBlocks = wrapper.findAllComponents({ name: 'CodeBlock' })
    expect(codeBlocks.length).toBe(2)

    const cargoBlock = codeBlocks[0]
    expect(cargoBlock.exists()).toBe(true)
    expect(cargoBlock.props('label')).toBe('Cargo.toml')
    expect(cargoBlock.props('code')).toContain('simple_ssh')
  })

  it('renders programmatic PTY code block with correct content', () => {
    const wrapper = mount(CodeExamplesSection)
    const codeBlocks = wrapper.findAllComponents({ name: 'CodeBlock' })
    expect(codeBlocks.length).toBe(2)

    const ptyBlock = codeBlocks[1]
    expect(ptyBlock?.exists()).toBe(true)
    expect(ptyBlock?.props('label')).toBe('Programmatic PTY')
    expect(ptyBlock?.props('code')).toContain('session')
  })

  it('both blocks have copy functionality', async () => {
    const wrapper = mount(CodeExamplesSection)
    const codeBlocks = wrapper.findAllComponents({ name: 'CodeBlock' })

    const copyButton1 = codeBlocks[0]?.find('.copy-btn')
    const copyButton2 = codeBlocks[1]?.find('.copy-btn')

    expect(copyButton1?.exists()).toBe(true)
    expect(copyButton2?.exists()).toBe(true)

    if (copyButton1 && copyButton2) {
      await copyButton1.trigger('click')
      await copyButton2.trigger('click')

      expect(navigator.clipboard.writeText).toHaveBeenCalledTimes(2)
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(codeBlocks[0].props('code'))
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(codeBlocks[1].props('code'))
    }
  })

  it('has scroll reveal animation on mount', () => {
    const wrapper = mount(CodeExamplesSection)

    const section = wrapper.find('.code-examples-section')
    expect(section.exists()).toBe(true)
    expect(section.classes()).not.toContain('visible')
  })
})
