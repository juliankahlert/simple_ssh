<script setup lang="ts">
import { ref, onUnmounted } from 'vue';
import GithubIcon from './icons/GithubIcon.vue';
import CopyIcon from './icons/CopyIcon.vue';
import DocsIcon from './icons/DocsIcon.vue';
import CratesIcon from './icons/CratesIcon.vue';

const activeTab = ref<'lib' | 'cli'>('lib');
const copied = ref(false);
let copyTimeout: ReturnType<typeof setTimeout> | null = null;

const snippets = {
  lib: 'cargo add simple_ssh',
  cli: 'cargo install simple_ssh --features cli'
};

async function copySnippet() {
  try {
    await navigator.clipboard.writeText(snippets[activeTab.value]);
    copied.value = true;
    if (copyTimeout) {
      clearTimeout(copyTimeout);
    }
    copyTimeout = setTimeout(() => {
      copied.value = false;
      copyTimeout = null;
    }, 2000);
  } catch (err: unknown) {
    console.error('Failed to copy:', err);
  }
}

onUnmounted(() => {
  if (copyTimeout) {
    clearTimeout(copyTimeout);
    copyTimeout = null;
  }
});
</script>

<template>
  <section class="install" id="install">
    <div class="install-card">
      <div class="section-header">
        <div class="section-label">Install</div>
        <h2 class="section-title">Get started in seconds.</h2>
        <p class="section-desc">Add simple_ssh to your project or install the CLI tools.</p>
      </div>

      <div class="install-tabs">
        <button
          class="install-tab"
          :class="{ active: activeTab === 'lib' }"
          @click="activeTab = 'lib'"
        >
          Library
        </button>
        <button
          class="install-tab"
          :class="{ active: activeTab === 'cli' }"
          @click="activeTab = 'cli'"
        >
          CLI Tool
        </button>
      </div>

      <div class="install-snippet" :class="{ copied }" @click="copySnippet">
        <div class="snippet-content">
          <span class="prompt">$</span>
          <span>{{ snippets[activeTab] }}</span>
          <span class="terminal-cursor">|</span>
        </div>
        <button class="copy-btn" :class="{ success: copied }" @click.stop="copySnippet">
          <CopyIcon :size="16" />
          <span v-if="copied">Copied!</span>
          <span v-else>Copy</span>
        </button>
      </div>

      <div class="install-links">
        <a href="https://github.com/juliankahlert/simple_ssh" target="_blank" rel="noopener noreferrer">
          <GithubIcon :size="16" />
          <span>GitHub</span>
        </a>
        <a href="https://docs.rs/simple_ssh" target="_blank" rel="noopener noreferrer">
          <DocsIcon :size="16" />
          <span>Documentation</span>
        </a>
        <a href="https://crates.io/crates/simple_ssh" target="_blank" rel="noopener noreferrer">
          <CratesIcon :size="16" />
          <span>Crates.io</span>
        </a>
      </div>
    </div>
  </section>
</template>

<style scoped>
.install {
  padding: var(--space-2xl) var(--space-xl);
  max-width: 700px;
  margin: 0 auto;
  text-align: center;
}

.install-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border-light);
  border-radius: 16px;
  padding: var(--space-2xl);
  position: relative;
  overflow: hidden;
}

.install-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  width: 200px;
  height: 1px;
  background: linear-gradient(90deg, transparent, var(--accent), transparent);
}

.install-card::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 50%;
  transform: translateX(-50%);
  width: 200px;
  height: 1px;
  background: linear-gradient(90deg, transparent, var(--accent), transparent);
}

.section-header {
  margin-bottom: var(--space-xl);
}

.install-tabs {
  display: flex;
  justify-content: center;
  gap: var(--space-xs);
  margin-bottom: var(--space-lg);
}

.install-tab {
  padding: var(--space-sm) var(--space-md);
  background: var(--bg-tertiary);
  border: 1px solid var(--border-light);
  border-radius: 6px;
  color: var(--text-secondary);
  font-family: var(--font-display);
  font-size: 0.85rem;
  cursor: pointer;
  transition: all 0.2s;
}

.install-tab:hover {
  border-color: var(--border-medium);
  color: var(--text-primary);
}

.install-tab.active {
  background: var(--accent);
  border-color: var(--accent);
  color: var(--bg-primary);
}

.install-snippet {
  display: inline-flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-lg);
  padding: var(--space-sm) var(--space-sm) var(--space-sm) var(--space-md);
  background: var(--bg-tertiary);
  border: 1px solid var(--border-light);
  border-radius: 8px;
  font-family: var(--font-mono);
  font-size: 0.9rem;
  color: var(--text-primary);
  cursor: pointer;
  transition: all 0.2s;
  max-width: 100%;
  overflow-x: auto;
}

.install-snippet:hover {
  border-color: var(--accent);
}

.install-snippet.copied {
  border-color: var(--success);
}

.snippet-content {
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.install-snippet .prompt {
  color: var(--accent);
  font-weight: 600;
}

.copy-btn {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  padding: var(--space-xs) var(--space-sm);
  background: var(--bg-elevated);
  border: 1px solid var(--border-light);
  border-radius: 4px;
  color: var(--text-secondary);
  font-size: 0.75rem;
  cursor: pointer;
  transition: all 0.2s;
  font-family: var(--font-display);
}

.copy-btn:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
  border-color: var(--border-medium);
}

.copy-btn.success {
  background: var(--success);
  border-color: var(--success);
  color: var(--bg-primary);
}

.install-links {
  display: flex;
  justify-content: center;
  gap: var(--space-xl);
  margin-top: var(--space-lg);
  flex-wrap: wrap;
}

.install-links a {
  color: var(--text-tertiary);
  font-size: 0.9rem;
  text-decoration: none;
  display: inline-flex;
  align-items: center;
  gap: var(--space-xs);
  transition: color 0.2s;
}

.install-links a:hover {
  color: var(--accent);
}

.install-links a svg {
  width: 16px;
  height: 16px;
}

.terminal-cursor {
  color: var(--accent);
  animation: blink-cursor 1s step-end infinite;
  font-weight: 300;
}

@keyframes blink-cursor {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.copy-btn.success {
  animation: copy-pulse 0.3s var(--ease-out);
}

@keyframes copy-pulse {
  50% { transform: scale(1.05); }
  100% { transform: scale(1); }
}

@media (max-width: 768px) {
  .install {
    padding: var(--space-xl) var(--space-md);
    max-width: 100%;
  }

  .install-card {
    padding: var(--space-xl) var(--space-md);
  }

  .install-snippet {
    font-size: 0.8rem;
    padding: var(--space-xs);
    width: 100%;
    justify-content: space-between;
  }
  
  .copy-btn span {
    display: none;
  }

  .install-links {
    flex-wrap: wrap;
    gap: var(--space-md);
  }
}
</style>
