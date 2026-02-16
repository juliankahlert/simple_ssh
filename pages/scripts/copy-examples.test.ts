import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import path from 'path';
import { fileURLToPath } from 'url';
import { copyExamples } from './copy-examples';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

vi.mock('fs-extra', () => ({
  copy: vi.fn(),
  ensureDir: vi.fn()
}));

vi.mock('fs/promises', async (importOriginal) => {
  const actual = await importOriginal<typeof import('fs/promises')>();
  return {
    ...actual,
    readFile: vi.fn().mockResolvedValue('fn main() {}'),
    writeFile: vi.fn().mockResolvedValue(undefined)
  };
});

import { copy, ensureDir } from 'fs-extra';

describe('copy-examples', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.clearAllTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('creates public/examples/ directory if it doesn\'t exist', async () => {
    ensureDir.mockResolvedValue(undefined);
    copy.mockResolvedValue(undefined);

    await copyExamples();

    expect(ensureDir).toHaveBeenCalledWith(path.join(__dirname, '../../public/examples/'));
  });

  it('copies all 4 .rs files from ../examples/', async () => {
    const filesToCopy = [
      'basic.rs',
      'scp.rs',
      'pty.rs',
      'ipv6.rs'
    ];

    ensureDir.mockResolvedValue(undefined);
    copy.mockResolvedValue(undefined);

    await copyExamples();

    const sourceDir = path.resolve(__dirname, '../../examples/');
    const targetDir = path.resolve(__dirname, '../../public/examples/');
    filesToCopy.forEach(file => {
      expect(copy).toHaveBeenCalledWith(
        path.join(sourceDir, file),
        path.join(targetDir, file)
      );
    });
  });

  it('copies all files in correct order', async () => {
    ensureDir.mockResolvedValue(undefined);
    copy.mockImplementation(() => Promise.resolve());

    await copyExamples();

    const sourceDir = path.resolve(__dirname, '../../examples/');
    const targetDir = path.resolve(__dirname, '../../public/examples/');

    expect(copy).toHaveBeenCalledTimes(4);
    expect(copy).toHaveBeenNthCalledWith(1, path.join(sourceDir, 'basic.rs'), path.join(targetDir, 'basic.rs'));
    expect(copy).toHaveBeenNthCalledWith(2, path.join(sourceDir, 'scp.rs'), path.join(targetDir, 'scp.rs'));
    expect(copy).toHaveBeenNthCalledWith(3, path.join(sourceDir, 'pty.rs'), path.join(targetDir, 'pty.rs'));
    expect(copy).toHaveBeenNthCalledWith(4, path.join(sourceDir, 'ipv6.rs'), path.join(targetDir, 'ipv6.rs'));
  });

  it('runs without errors', async () => {
    ensureDir.mockResolvedValue(undefined);
    copy.mockResolvedValue(undefined);

    await expect(copyExamples()).resolves.toBeUndefined();
  });

  it('handles copy errors gracefully', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    ensureDir.mockResolvedValue(undefined);
    copy.mockRejectedValue(new Error('Copy failed'));

    await expect(copyExamples()).rejects.toThrow();
    expect(consoleSpy).toHaveBeenCalledWith('Failed to copy examples:', expect.any(Error));

    consoleSpy.mockRestore();
  });

  it('calls ensureDir before copy operations', async () => {
    copy.mockImplementation(() => Promise.resolve());
    ensureDir.mockImplementation(() => Promise.resolve());

    await copyExamples();

    expect(ensureDir.mock.invocationCallOrder[0]).toBeLessThan(copy.mock.invocationCallOrder[0]);
  });

  it('handles empty examples directory', async () => {
    ensureDir.mockResolvedValue(undefined);
    copy.mockRejectedValue(new Error('No such file'));

    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    await expect(copyExamples()).rejects.toThrow();
    expect(consoleSpy).toHaveBeenCalled();

    consoleSpy.mockRestore();
  });
});
