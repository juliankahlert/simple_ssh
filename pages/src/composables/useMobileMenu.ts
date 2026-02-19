import { ref, watch, onBeforeUnmount, type Ref } from 'vue';

export function useMobileMenu() {
  const isOpen: Ref<boolean> = ref(false);

  watch(isOpen, (open) => {
    if (typeof window !== 'undefined') {
      document.body.style.overflow = open ? 'hidden' : '';
    }
  }, { flush: 'sync' });

  onBeforeUnmount(() => {
    if (typeof window !== 'undefined') {
      document.body.style.overflow = '';
    }
  });

  function toggle() {
    isOpen.value = !isOpen.value;
  }

  function close() {
    isOpen.value = false;
  }

  return {
    isOpen,
    toggle,
    close
  };
}