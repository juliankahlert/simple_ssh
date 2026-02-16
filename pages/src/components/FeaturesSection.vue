<template>
  <section class="features" id="features">
    <div class="section-header reveal">
      <div class="section-label">Features</div>
      <h2 class="section-title">Everything you need for SSH in Rust.</h2>
      <p class="section-desc">A complete toolkit for SSH operations with an ergonomic builder API designed for async Rust.</p>
    </div>

    <div class="features-grid reveal">
      <slot />
    </div>
  </section>
</template>

<script setup lang="ts">
import { useScrollReveal } from '../composables/useScrollReveal';

useScrollReveal('.features .reveal', {
  threshold: 0.1
});
</script>

<style scoped>
.features {
  padding: var(--space-xl) var(--space-xl);
  max-width: 1200px;
  margin: 0 auto;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.section-header {
  margin-bottom: var(--space-lg);
}

.section-title {
  font-size: clamp(1.75rem, 4vw, 2.5rem);
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-md);
}

.reveal {
  opacity: 0;
  transform: translateY(20px);
  transition: opacity 0.7s var(--ease-out), transform 0.7s var(--ease-out);
}

.reveal.visible {
  opacity: 1;
  transform: translateY(0);
}

.features-grid.reveal {
  transition-delay: 0.1s;
}

@media (max-width: 1024px) {
  .features-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 768px) {
  .features {
    padding: var(--space-lg) var(--space-md);
    min-height: auto;
  }

  .features-grid {
    grid-template-columns: 1fr;
    gap: var(--space-sm);
  }
}
</style>
