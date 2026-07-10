import { describe, it, expect } from 'vitest';
import { useExamples, type ExampleFile } from './useExamples';

describe('useExamples', () => {
  it('composable returns expected structure', () => {
    const result = useExamples();

    expect(result).toHaveProperty('examples');
    expect(result).toHaveProperty('currentExample');
    expect(result).toHaveProperty('activeExample');

    expect(Array.isArray(result.examples.value)).toBe(true);
    expect(typeof result.currentExample.value === 'string').toBe(true);
    expect(typeof result.activeExample.value === 'object' || result.activeExample.value === undefined).toBe(true);
  });

  it('examples array is populated from static data', () => {
    const { examples } = useExamples();
    expect(examples.value.length).toBe(4);
    expect(examples.value[0].name).toBe('basic.rs');
    expect(examples.value[1].name).toBe('scp.rs');
    expect(examples.value[2].name).toBe('pty.rs');
    expect(examples.value[3].name).toBe('ipv6.rs');
  });

  it('currentExample defaults to basic.rs', () => {
    const { currentExample } = useExamples();
    expect(currentExample.value).toBe('basic.rs');
  });

  it('activeExample returns the matching example file', () => {
    const { currentExample, activeExample } = useExamples();
    expect(activeExample.value).toBeDefined();
    expect(activeExample.value?.name).toBe('basic.rs');
  });

  it('activeExample returns undefined when no matching file exists', () => {
    const { currentExample, activeExample, examples } = useExamples();
    examples.value = [];
    currentExample.value = 'basic.rs';

    expect(activeExample.value).toBeUndefined();
  });

  it('changing currentExample updates activeExample', () => {
    const { currentExample, activeExample } = useExamples();
    
    expect(activeExample.value?.name).toBe('basic.rs');
    
    currentExample.value = 'pty.rs';
    expect(activeExample.value?.name).toBe('pty.rs');
  });

  it('each example has required properties', () => {
    const { examples } = useExamples();
    
    examples.value.forEach(example => {
      expect(example).toHaveProperty('name');
      expect(example).toHaveProperty('path');
      expect(example).toHaveProperty('content');
      expect(example).toHaveProperty('language');
    });
  });

  it('examples array is reactive', () => {
    const { examples } = useExamples();
    
    expect(examples.value.length).toBe(4);
    
    examples.value.push({
      name: 'test.rs',
      path: 'test.rs',
      content: 'fn main() {}',
      language: 'rust'
    });
    
    expect(examples.value.length).toBe(5);
  });
});
