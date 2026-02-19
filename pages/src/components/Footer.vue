<script setup lang="ts">
import type { Component } from 'vue';
import { GithubIcon, GlobeIcon, FileIcon } from './icons';

const footerLinks = [
  {
    name: 'GitHub',
    url: 'https://github.com/juliankahlert/simple_ssh'
  },
  {
    name: 'Docs',
    url: 'https://docs.rs/simple_ssh'
  },
  {
    name: 'Crates.io',
    url: 'https://crates.io/crates/simple_ssh'
  }
];

const ICONS: Record<string, Component> = {
  'GitHub': GithubIcon,
  'Docs': FileIcon,
  'Crates.io': GlobeIcon
};

function getIcon(name: string): Component {
  return ICONS[name] || GithubIcon;
}
</script>

<template>
  <footer class="footer">
    <div class="footer-content">
      <div class="footer-left">
        <span class="footer-logo">&gt; simple_ssh</span>
        <span class="footer-license">MIT Licensed</span>
      </div>

      <div class="footer-right">
        <a
          v-for="link in footerLinks"
          :key="link.name"
          :href="link.url"
          target="_blank"
          rel="noopener noreferrer"
          class="footer-link"
        >
          <component :is="getIcon(link.name)" :size="20" />
          {{ link.name }}
        </a>
      </div>
    </div>
  </footer>
</template>

<style scoped>
.footer {
  border-top: 1px solid var(--border-light);
  padding: var(--space-lg);
  background: var(--bg-secondary);
}

.footer-content {
  max-width: 1000px;
  margin: 0 auto;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-lg);
}

.footer-left {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  flex-wrap: wrap;
}

.footer-logo {
  font-family: var(--font-mono);
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
}

.footer-license {
  font-size: 0.875rem;
  color: var(--text-tertiary);
}

.footer-right {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
}

.footer-link {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-md);
  background: var(--bg-elevated);
  border: 1px solid var(--border-light);
  border-radius: 6px;
  color: var(--text-secondary);
  text-decoration: none;
  font-size: 0.875rem;
  font-weight: 500;
  transition: all 0.2s ease;
}

.footer-link:hover {
  background: var(--bg-secondary);
  color: var(--accent);
  border-color: var(--accent);
}

.footer-link:active {
  transform: scale(0.98);
}

.footer-link:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

@media (max-width: 768px) {
  .footer {
    padding: var(--space-md);
  }

  .footer-content {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-md);
  }

  .footer-left,
  .footer-right {
    width: 100%;
    justify-content: space-between;
  }

  .footer-right {
    justify-content: flex-start;
  }
}

@media (max-width: 480px) {
  .footer-content {
    gap: var(--space-md);
  }

  .footer-left {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-sm);
  }

  .footer-logo {
    font-size: 0.875rem;
  }

  .footer-license {
    font-size: 0.8125rem;
  }

  .footer-link {
    padding: var(--space-xs) var(--space-sm);
    font-size: 0.8125rem;
  }

  .footer-right {
    gap: var(--space-sm);
  }
}
</style>
