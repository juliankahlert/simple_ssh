import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useScrollReveal } from './useScrollReveal';
import { mount } from '@vue/test-utils';

describe('useScrollReveal', () => {
  let observeSpy: ReturnType<typeof vi.fn>;
  let unobserveSpy: ReturnType<typeof vi.fn>;
  let disconnectSpy: ReturnType<typeof vi.fn>;
  let addClassSpy: ReturnType<typeof vi.fn>;

  let originalIntersectionObserver: typeof globalThis.IntersectionObserver;
  let originalQuerySelectorAll: typeof document.querySelectorAll;

  beforeEach(() => {
    originalIntersectionObserver = globalThis.IntersectionObserver;
    originalQuerySelectorAll = document.querySelectorAll;
    observeSpy = vi.fn();
    unobserveSpy = vi.fn();
    disconnectSpy = vi.fn();
    addClassSpy = vi.fn();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    globalThis.IntersectionObserver = originalIntersectionObserver;
    document.querySelectorAll = originalQuerySelectorAll;
  });

  it('creates IntersectionObserver with default options', async () => {
    let capturedOptions: IntersectionObserverInit = {};

    class MockIntersectionObserver {
      observe = observeSpy;
      unobserve = unobserveSpy;
      disconnect = disconnectSpy;

      constructor(
        _callback: IntersectionObserverCallback,
        options?: IntersectionObserverInit
      ) {
        capturedOptions = options || {};
      }
    }

    const TestComponent = {
      template: '<div class="reveal">Test</div>',
      setup() {
        useScrollReveal('.reveal');
        return {};
      }
    };

    globalThis.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

    const wrapper = mount(TestComponent);
    await wrapper.vm.$nextTick();

    expect(capturedOptions.threshold).toBe(0.1);
    expect(capturedOptions.rootMargin).toBe('0px 0px -50px 0px');

    wrapper.unmount();
    await wrapper.vm.$nextTick();
  });

  it('adds visible class when element enters viewport', async () => {
    let capturedCallback: IntersectionObserverCallback | null = null;

    class MockIntersectionObserver {
      observe = observeSpy;
      unobserve = unobserveSpy;
      disconnect = disconnectSpy;

      constructor(callback: IntersectionObserverCallback, _options?: IntersectionObserverInit) {
        capturedCallback = callback;
      }
    }

    const TestComponent = {
      template: '<div class="reveal">Test</div>',
      setup() {
        useScrollReveal('.reveal');
        return {};
      }
    };

    const mockElement = {
      classList: {
        add: addClassSpy,
        contains: vi.fn().mockReturnValue(false)
      }
    };

    document.querySelectorAll = vi.fn().mockReturnValue([mockElement] as unknown as NodeListOf<Element>);

    globalThis.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

    const wrapper = mount(TestComponent);
    await wrapper.vm.$nextTick();

    const mockEntry: Partial<IntersectionObserverEntry> = {
      isIntersecting: true,
      target: mockElement
    };

    capturedCallback!([mockEntry as IntersectionObserverEntry], {} as IntersectionObserver);

    expect(addClassSpy).toHaveBeenCalledWith('visible');

    wrapper.unmount();
  });

  it('disconnects observer on unmount', async () => {
    class MockIntersectionObserver {
      observe = observeSpy;
      unobserve = unobserveSpy;
      disconnect = disconnectSpy;

      constructor(_callback: IntersectionObserverCallback, _options?: IntersectionObserverInit) {}
    }

    const TestComponent = {
      template: '<div class="reveal">Test</div>',
      setup() {
        useScrollReveal('.reveal');
        return {};
      }
    };

    document.querySelectorAll = vi.fn().mockReturnValue([]);

    globalThis.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

    const wrapper = mount(TestComponent);
    await wrapper.vm.$nextTick();

    wrapper.unmount();
    await wrapper.vm.$nextTick();

    expect(disconnectSpy).toHaveBeenCalled();
  });

  it('uses custom threshold', async () => {
    let capturedOptions: IntersectionObserverInit = {};

    class MockIntersectionObserver {
      observe = observeSpy;
      unobserve = unobserveSpy;
      disconnect = disconnectSpy;

      constructor(_callback: IntersectionObserverCallback, options?: IntersectionObserverInit) {
        capturedOptions = options || {};
      }
    }

    const TestComponent = {
      template: '<div class="reveal">Test</div>',
      setup() {
        useScrollReveal('.reveal', { threshold: 0.5 });
        return {};
      }
    };

    document.querySelectorAll = vi.fn().mockReturnValue([]);

    globalThis.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

    const wrapper = mount(TestComponent);
    await wrapper.vm.$nextTick();

    expect(capturedOptions.threshold).toBe(0.5);

    wrapper.unmount();
  });

  it('uses custom rootMargin', async () => {
    let capturedOptions: IntersectionObserverInit = {};

    class MockIntersectionObserver {
      observe = observeSpy;
      unobserve = unobserveSpy;
      disconnect = disconnectSpy;

      constructor(_callback: IntersectionObserverCallback, options?: IntersectionObserverInit) {
        capturedOptions = options || {};
      }
    }

    const TestComponent = {
      template: '<div class="reveal">Test</div>',
      setup() {
        useScrollReveal('.reveal', { rootMargin: '0px 0px -100px 0px' });
        return {};
      }
    };

    document.querySelectorAll = vi.fn().mockReturnValue([]);

    globalThis.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

    const wrapper = mount(TestComponent);
    await wrapper.vm.$nextTick();

    expect(capturedOptions.rootMargin).toBe('0px 0px -100px 0px');

    wrapper.unmount();
  });

  it('observes multiple .reveal elements', async () => {
    class MockIntersectionObserver {
      observe = observeSpy;
      unobserve = unobserveSpy;
      disconnect = disconnectSpy;

      constructor(_callback: IntersectionObserverCallback, _options?: IntersectionObserverInit) {}
    }

    const TestComponent = {
      template: '<div><div class="reveal">Test1</div><div class="reveal">Test2</div><div class="reveal">Test3</div></div>',
      setup() {
        useScrollReveal('.reveal');
        return {};
      }
    };

    const mockElements = [
      { classList: { add: addClassSpy, contains: vi.fn().mockReturnValue(false) } },
      { classList: { add: addClassSpy, contains: vi.fn().mockReturnValue(false) } },
      { classList: { add: addClassSpy, contains: vi.fn().mockReturnValue(false) } }
    ];

    document.querySelectorAll = vi.fn().mockReturnValue(mockElements as unknown as NodeListOf<Element>);

    globalThis.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

    const wrapper = mount(TestComponent);
    await wrapper.vm.$nextTick();

    expect(observeSpy).toHaveBeenCalledTimes(3);

    wrapper.unmount();
  });

  it('unobserves element after it becomes visible', async () => {
    let capturedCallback: IntersectionObserverCallback | null = null;

    class MockIntersectionObserver {
      observe = observeSpy;
      unobserve = unobserveSpy;
      disconnect = disconnectSpy;

      constructor(callback: IntersectionObserverCallback, _options?: IntersectionObserverInit) {
        capturedCallback = callback;
      }
    }

    const TestComponent = {
      template: '<div class="reveal">Test</div>',
      setup() {
        useScrollReveal('.reveal');
        return {};
      }
    };

    const mockElement = {
      classList: {
        add: addClassSpy,
        contains: vi.fn().mockReturnValue(false)
      }
    };

    document.querySelectorAll = vi.fn().mockReturnValue([mockElement] as unknown as NodeListOf<Element>);

    globalThis.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

    const wrapper = mount(TestComponent);
    await wrapper.vm.$nextTick();

    const mockEntry: Partial<IntersectionObserverEntry> = {
      isIntersecting: true,
      target: mockElement
    };

    capturedCallback!([mockEntry as IntersectionObserverEntry], {} as IntersectionObserver);

    expect(unobserveSpy).toHaveBeenCalledWith(mockElement);

    wrapper.unmount();
  });
});