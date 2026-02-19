<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" :class="iconClass">
    <polyline points="9 18 15 12 9 6"/>
  </svg>
</template>

<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  size?: number;
  class?: string;
}
const props = withDefaults(defineProps<Props>(), {
  size: 24
});
const iconClass = computed(() => props.class);
</script>
