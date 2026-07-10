<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" focusable="false" :class="iconClass">
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
    <polyline points="14 2 14 8 20 8" fill="none" stroke="currentColor" stroke-width="2"/>
  </svg>
</template>

<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  size?: number;
  class?: string;
}
const props = withDefaults(defineProps<Props>(), {
  size: 24,
  class: ''
});
const iconClass = computed(() => props.class);
</script>
