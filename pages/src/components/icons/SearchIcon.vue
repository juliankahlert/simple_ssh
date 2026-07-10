<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" :class="iconClass" :aria-label="ariaLabel" :aria-hidden="!ariaLabel" focusable="false">
    <circle cx="11" cy="11" r="8"/>
    <line x1="21" y1="21" x2="16.65" y2="16.65"/>
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
  size: 24,
  ariaLabel: ''
});
const iconClass = computed(() => props.class);
</script>
