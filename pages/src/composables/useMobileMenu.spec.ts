import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { useMobileMenu } from './useMobileMenu';

describe('useMobileMenu', () => {
  let bodyOverflow = '';
  let originalDescriptor: PropertyDescriptor | undefined;

  beforeEach(() => {
    originalDescriptor = Object.getOwnPropertyDescriptor(document.body.style, 'overflow');
    Object.defineProperty(document.body.style, 'overflow', {
      configurable: true,
      get: () => bodyOverflow,
      set: (v) => { bodyOverflow = v; }
    });
    bodyOverflow = '';
  });

  afterEach(() => {
    if (originalDescriptor) {
      Object.defineProperty(document.body.style, 'overflow', originalDescriptor);
    } else {
      document.body.style.removeProperty('overflow');
    }
    bodyOverflow = '';
  });

  it('composable returns expected structure', () => {
    const result = useMobileMenu();

    expect(result).toHaveProperty('isOpen');
    expect(result).toHaveProperty('toggle');
    expect(result).toHaveProperty('close');

    expect(typeof result.isOpen.value).toBe('boolean');
    expect(typeof result.toggle).toBe('function');
    expect(typeof result.close).toBe('function');
  });

  it('isOpen defaults to false', () => {
    const { isOpen } = useMobileMenu();
    expect(isOpen.value).toBe(false);
  });

  it('toggle switches state from false to true', () => {
    const { isOpen, toggle } = useMobileMenu();
    expect(isOpen.value).toBe(false);

    toggle();
    expect(isOpen.value).toBe(true);
  });

  it('toggle switches state from true to false', () => {
    const { isOpen, toggle } = useMobileMenu();
    isOpen.value = true;

    toggle();
    expect(isOpen.value).toBe(false);
  });

  it('close sets isOpen to false', () => {
    const { isOpen, close } = useMobileMenu();
    isOpen.value = true;

    close();
    expect(isOpen.value).toBe(false);
  });

  it('body overflow is hidden when open', () => {
    const { isOpen, toggle } = useMobileMenu();
    toggle();
    expect(isOpen.value).toBe(true);
    expect(document.body.style.overflow).toBe('hidden');
  });

  it('body overflow is restored when closed via close()', () => {
    const { isOpen, close } = useMobileMenu();
    isOpen.value = true;
    expect(document.body.style.overflow).toBe('hidden');

    close();
    expect(document.body.style.overflow).toBe('');
  });

  it('body overflow is restored when closed via toggle()', () => {
    const { isOpen, toggle } = useMobileMenu();
    isOpen.value = true;
    expect(document.body.style.overflow).toBe('hidden');

    toggle();
    expect(isOpen.value).toBe(false);
    expect(document.body.style.overflow).toBe('');
  });
});