import { ref, computed, type ComputedRef } from 'vue';
import { examplesData } from './examplesData';

export interface ExampleFile {
  name: string;
  path: string;
  content: string;
  language: string;
}

export function useExamples() {
  const examples = ref<ExampleFile[]>(structuredClone(examplesData));
  const currentExample = ref<string>('basic.rs');

  const activeExample: ComputedRef<ExampleFile | undefined> = computed(() => {
    return examples.value.find(ex => ex.name === currentExample.value);
  });

  return {
    examples,
    currentExample,
    activeExample
  };
}
