<script setup lang="ts">
import VSCodeEditor from './VSCodeEditor.vue';
import { useExamples } from '../composables/useExamples';
import { ref, onMounted, onUnmounted } from 'vue';

const { examples, currentExample } = useExamples();

const sectionRef = ref<HTMLElement | null>(null);
const isVisible = ref(false);
let observer: IntersectionObserver | null = null;

onMounted(() => {
  if (typeof IntersectionObserver === 'undefined') {
    isVisible.value = true;
    return;
  }

  observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          isVisible.value = true;
          observer?.disconnect();
        }
      });
    },
    { threshold: 0.1 }
  );

  if (sectionRef.value) {
    observer.observe(sectionRef.value);
  }
});

onUnmounted(() => {
  observer?.disconnect();
  observer = null;
});

function handleTabChange(name: string) {
  currentExample.value = name;
}
</script>

<template>
  <div class="editor-section" ref="sectionRef" :class="{ visible: isVisible }">
    <VSCodeEditor
      :examples="examples"
      :active-example="currentExample"
      :should-animate="isVisible"
      @tab-change="handleTabChange"
    />
  </div>
</template>

<style scoped>
.editor-section {
  padding: var(--space-xl);
  max-width: 900px;
  margin: 0 auto;
  opacity: 0;
  transform: translateY(20px) scale(0.98);
  transition: opacity 0.8s var(--ease-out), transform 0.8s var(--ease-out) 0.4s;
}

.editor-section.visible {
  opacity: 1;
  transform: translateY(0) scale(1);
}

@media (max-width: 768px) {
  .editor-section {
    padding: var(--space-xl) var(--space-md);
    max-width: 100%;
  }
}
</style>
