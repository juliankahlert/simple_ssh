<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" :class="iconClass" :aria-label="ariaLabel" :aria-hidden="!ariaLabel" focusable="false">
    <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/>
    <polyline points="13 2 13 9 20 9"/>
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
