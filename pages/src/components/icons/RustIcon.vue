<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="currentColor" :class="iconClass">
    <path d="M12 15a3 3 0 100-6 3 3 0 000 6z"/>
    <path fill-rule="evenodd" d="M9.024 2.783a1 1 0 01.976-.783h4a1 1 0 01.976.783l.353 1.59a7.963 7.963 0 011.462.605l1.39-.834a1 1 0 011.258.126l2.828 2.828a1 1 0 01.126 1.258l-.834 1.39c.26.458.467.948.605 1.462l1.59.353a1 1 0 01.783.976v4a1 1 0 01-.783.976l-1.59.353a7.963 7.963 0 01-.605 1.462l.834 1.39a1 1 0 01-.126 1.258l-2.828 2.828a1 1 0 01-1.258.126l-1.39-.834a7.963 7.963 0 01-1.462.605l-.353 1.59a1 1 0 01-.976.783h-4a1 1 0 01-.976-.783l-.353-1.59a7.962 7.962 0 01-1.462-.605l-1.39.834a1 1 0 01-1.258-.126L2.733 18.6a1 1 0 01-.126-1.258l.834-1.39a7.962 7.962 0 01-.605-1.462l-1.59-.353A1 1 0 01.463 13.5v-4a1 1 0 01.783-.976l1.59-.353a7.963 7.963 0 01.605-1.462l-.834-1.39a1 1 0 01.126-1.258l2.828-2.828a1 1 0 011.258-.126l1.39.834a7.963 7.963 0 011.462-.605l.353-1.59zM12 17a5 5 0 100-10 5 5 0 000 10z" clip-rule="evenodd"/>
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
