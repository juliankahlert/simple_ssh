<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" :class="iconClass" :aria-label="ariaLabel" :aria-hidden="!ariaLabel" focusable="false">
    <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
    <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
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
