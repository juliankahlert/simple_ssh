import { ref, type Ref } from 'vue';

export function useClipboard(timeout = 1500) {
  const copied: Ref<boolean> = ref(false);
  let resetTimeout: ReturnType<typeof setTimeout> | null = null;

  async function copy(text: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
      copied.value = true;
      if (resetTimeout) {
        clearTimeout(resetTimeout);
      }
      resetTimeout = setTimeout(() => {
        copied.value = false;
        resetTimeout = null;
      }, timeout);
    } catch (error: unknown) {
      console.error(error);
    }
  }

  return {
    copied,
    copy
  };
}
