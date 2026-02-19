import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import CodeBlock from './CodeBlock.vue'

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
  vi.useRealTimers();
  vi.restoreAllMocks();
  if (originalClipboard) {
    Object.defineProperty(navigator, 'clipboard', originalClipboard);
  } else {
    delete (navigator as any).clipboard;
  }
})

describe('CodeBlock', () => {
  it('renders label correctly', () => {
    const wrapper = mount(CodeBlock, {
      props: {
        label: 'Cargo.toml',
        code: '[package]\nname = "test"'
      }
    })
    expect(wrapper.find('.label').text()).toBe('Cargo.toml')
  })

  it('renders code content correctly', () => {
    const wrapper = mount(CodeBlock, {
      props: {
        label: 'Test',
        code: 'console.log("hello")'
      }
    })
    expect(wrapper.find('.code-content').text()).toBe('console.log("hello")')
  })

  it('applies language class to code element', () => {
    const wrapper = mount(CodeBlock, {
      props: {
        label: 'Test',
        code: 'const x = 1',
        language: 'javascript'
      }
    })
    const codeContent = wrapper.find('.code-content')
    expect(codeContent.exists()).toBe(true)
    expect(codeContent.classes()).toContain('language-javascript')
  })

  it('defaults to text language when not specified', () => {
    const wrapper = mount(CodeBlock, {
      props: {
        label: 'Test',
        code: 'some code'
      }
    })
    const codeContent = wrapper.find('.code-content')
    expect(codeContent.exists()).toBe(true)
    expect(codeContent.classes()).toContain('language-text')
  })

  it('copies text to clipboard when button is clicked', async () => {
    const wrapper = mount(CodeBlock, {
      props: {
        label: 'Test',
        code: 'test code to copy'
      }
    })
    await wrapper.find('.copy-btn').trigger('click')
    expect((navigator.clipboard as any).writeText).toHaveBeenCalledWith('test code to copy')
  })

  it('shows success state after copy', async () => {
    const wrapper = mount(CodeBlock, {
      props: {
        label: 'Test',
        code: 'test code'
      }
    })
    expect(wrapper.find('.copy-btn').classes()).not.toContain('success')
    await wrapper.find('.copy-btn').trigger('click')
    expect(wrapper.find('.copy-btn').classes()).toContain('success')
    expect(wrapper.find('.copy-btn span').text()).toBe('Copied!')
  })

  it('resets success state after 1.5 seconds', async () => {
    vi.useFakeTimers()
    const wrapper = mount(CodeBlock, {
      props: {
        label: 'Test',
        code: 'test code'
      }
    })
    await wrapper.find('.copy-btn').trigger('click')
    expect(wrapper.find('.copy-btn').classes()).toContain('success')

    vi.advanceTimersByTime(1500)
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.copy-btn').classes()).not.toContain('success')
    expect(wrapper.find('.copy-btn span').text()).toBe('Copy')
  })

  it('handles clipboard errors gracefully', async () => {
    ;(navigator.clipboard as any).writeText.mockRejectedValueOnce(new Error('Clipboard error'))
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const wrapper = mount(CodeBlock, {
      props: {
        label: 'Test',
        code: 'test code'
      }
    })
    await wrapper.find('.copy-btn').trigger('click')
    expect(consoleSpy).toHaveBeenCalledWith('Failed to copy:', expect.any(Error))
    expect(wrapper.find('.copy-btn').classes()).not.toContain('success')
    consoleSpy.mockRestore()
  })
})
