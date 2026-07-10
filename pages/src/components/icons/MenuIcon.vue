<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" :class="iconClass" :aria-label="ariaLabel" :aria-hidden="!ariaLabel" focusable="false">
    <line x1="4" y1="6" x2="20" y2="6"/>
    <line x1="4" y1="12" x2="20" y2="12"/>
    <line x1="4" y1="18" x2="20" y2="18"/>
  </svg>
</template>

<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  size?: number;
  class?: string;
  ariaLabel?: string;
}
const props = withDefaults(defineProps<Props>(), {
  size: 24
});
const iconClass = computed(() => props.class);
</script>
