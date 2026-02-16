import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useClipboard } from './useClipboard';

describe('useClipboard', () => {
  let originalClipboard: Navigator['clipboard'] | undefined;

  beforeEach(() => {
    originalClipboard = navigator.clipboard;
    Object.defineProperty(navigator, 'clipboard', {
      value: {
        writeText: vi.fn().mockResolvedValue(undefined)
      },
      configurable: true
    });
    vi.useFakeTimers();
  });

  afterEach(() => {
    if (originalClipboard !== undefined) {
      Object.defineProperty(navigator, 'clipboard', {
        value: originalClipboard,
        configurable: true
      });
    } else {
      delete (navigator as unknown as Record<string, unknown>)['clipboard'];
    }
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('returns copied ref defaulting to false', () => {
    const { copied } = useClipboard();
    expect(copied.value).toBe(false);
  });

  it('returns copy function', () => {
    const { copy } = useClipboard();
    expect(typeof copy).toBe('function');
  });

  it('copy calls navigator.clipboard.writeText', async () => {
    const { copy } = useClipboard();
    await copy('test text');
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('test text');
  });

  it('copied becomes true after successful copy', async () => {
    const { copied, copy } = useClipboard();
    expect(copied.value).toBe(false);

    await copy('test text');
    expect(copied.value).toBe(true);
  });

  it('copied resets to false after 1500ms timeout', async () => {
    const { copied, copy } = useClipboard();
    await copy('test text');
    expect(copied.value).toBe(true);

    vi.advanceTimersByTime(1500);
    expect(copied.value).toBe(false);
  });

  it('does not reset copied before timeout', async () => {
    const { copied, copy } = useClipboard();
    await copy('test text');
    expect(copied.value).toBe(true);

    vi.advanceTimersByTime(1000);
    expect(copied.value).toBe(true);

    vi.advanceTimersByTime(499);
    expect(copied.value).toBe(true);
  });

  it('clears previous timer when copy is called again', async () => {
    const { copied, copy } = useClipboard();
    await copy('first');
    expect(copied.value).toBe(true);

    vi.advanceTimersByTime(1000);
    await copy('second');
    expect(copied.value).toBe(true);

    vi.advanceTimersByTime(1000);
    expect(copied.value).toBe(true);

    vi.advanceTimersByTime(500);
    expect(copied.value).toBe(false);
  });

  it('handles clipboard API errors gracefully', async () => {
    const clipboardMock = {
      writeText: vi.fn().mockRejectedValue(new Error('Clipboard error'))
    };
    Object.defineProperty(navigator, 'clipboard', {
      value: clipboardMock,
      configurable: true
    });
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { copied, copy } = useClipboard();
    expect(copied.value).toBe(false);

    await copy('test text');
    expect(copied.value).toBe(false);
    expect(consoleSpy).toHaveBeenCalled();
  });
});
