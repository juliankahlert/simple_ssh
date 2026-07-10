<script setup lang="ts">
import type { Component } from 'vue';

interface Props {
  icon: Component;
  title: string;
  description: string;
  index?: number;
}

const props = withDefaults(defineProps<Props>(), {
  index: 0
});
</script>

<template>
  <div class="feature-card" :style="{ '--index': props.index }">
    <div class="icon-container">
      <component :is="props.icon" :size="36" class="icon" />
    </div>
    <h3 class="title">{{ props.title }}</h3>
    <p class="description">{{ props.description }}</p>
  </div>
</template>

<style scoped>
.feature-card {
  padding: var(--space-md);
  background: var(--bg-secondary);
  border-radius: 12px;
  border: 1px solid var(--border-light);
  transition: background 0.3s ease, border-color 0.3s ease, transform 0.3s ease;
  position: relative;
  overflow: hidden;
  transition-delay: calc(var(--index) * 0.08s);
}

.feature-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 2px;
  background: linear-gradient(90deg, var(--accent), var(--accent-bright));
  transform: scaleX(0);
  transform-origin: left;
  transition: transform 0.3s var(--ease-out);
}

.feature-card:hover {
  background: var(--bg-tertiary);
  transform: translateY(-2px);
}

.feature-card:hover::before {
  transform: scaleX(1);
}

.feature-card:hover .icon-container {
  border-color: var(--accent);
}

.icon-container {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border: 1px solid var(--border-light);
  border-radius: 10px;
  margin-bottom: var(--space-sm);
  background: linear-gradient(135deg, var(--bg-elevated), var(--bg-tertiary));
  transition: border-color 0.3s ease;
}

.icon {
  color: var(--accent);
}

.title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 var(--space-xs) 0;
}

.description {
  font-size: 0.875rem;
  line-height: 1.5;
  color: var(--text-secondary);
  margin: 0;
}
</style>
