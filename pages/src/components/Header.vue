<template>
  <header class="header" :class="{ scrolled: isScrolled }">
    <div class="header-container">
      <a href="/" class="logo">
        <span class="logo-mark">&gt;</span>
        <span class="logo-text">simple_ssh</span>
      </a>

      <nav class="nav-desktop">
        <a href="#features" class="nav-link">Features</a>
        <a href="#examples" class="nav-link">Examples</a>
        <a href="https://docs.rs/simple_ssh" class="nav-link" target="_blank" rel="noopener noreferrer">Docs</a>
        <a href="https://github.com/juliankahlert/simple_ssh" class="nav-link github-link" target="_blank" rel="noopener noreferrer">
          <GithubIcon :size="18" />
          <span>GitHub</span>
        </a>
      </nav>

      <button class="mobile-menu-btn" @click="toggleMobileMenu" :aria-label="isMobileMenuOpen ? 'Close menu' : 'Open menu'" :aria-expanded="isMobileMenuOpen" aria-controls="mobile-menu">
        <MenuIcon v-if="!isMobileMenuOpen" :size="24" />
        <span v-else class="close-icon">×</span>
      </button>
    </div>

    <Transition name="slide">
      <nav v-if="isMobileMenuOpen" class="nav-mobile" id="mobile-menu">
        <a href="#features" class="nav-link" @click="closeMobileMenu">Features</a>
        <a href="#examples" class="nav-link" @click="closeMobileMenu">Examples</a>
        <a href="https://docs.rs/simple_ssh" class="nav-link" target="_blank" rel="noopener noreferrer" @click="closeMobileMenu">Docs</a>
        <a href="https://github.com/juliankahlert/simple_ssh" class="nav-link github-link" target="_blank" rel="noopener noreferrer" @click="closeMobileMenu">
          <GithubIcon :size="18" />
          <span>GitHub</span>
        </a>
      </nav>
    </Transition>
  </header>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import GithubIcon from './icons/GithubIcon.vue';
import MenuIcon from './icons/MenuIcon.vue';

const isMobileMenuOpen = ref(false);
const isScrolled = ref(false);

function toggleMobileMenu(e: Event) {
  e.stopPropagation();
  isMobileMenuOpen.value = !isMobileMenuOpen.value;
  if (isMobileMenuOpen.value) {
    document.body.style.overflow = 'hidden';
  } else {
    document.body.style.overflow = '';
  }
}

function closeMobileMenu() {
  isMobileMenuOpen.value = false;
  document.body.style.overflow = '';
}

function handleClickOutside(event: MouseEvent) {
  if (isMobileMenuOpen.value) {
    const target = event.target as HTMLElement;
    if (target.closest('.mobile-menu-btn')) {
      return;
    }
    if (!target.closest('.header')) {
      closeMobileMenu();
    }
  }
}

function handleScroll() {
  isScrolled.value = window.scrollY > 50;
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside);
  window.addEventListener('scroll', handleScroll, { passive: true });
  handleScroll();
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
  window.removeEventListener('scroll', handleScroll);
  document.body.style.overflow = '';
  isScrolled.value = false;
});
</script>

<style scoped>
.header {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 100;
  background: transparent;
  backdrop-filter: blur(0px);
  border-bottom: 1px solid transparent;
  transition: background 0.3s ease, backdrop-filter 0.3s ease, border-color 0.3s ease;
}

.header.scrolled {
  background: rgba(12, 16, 22, 0.85);
  backdrop-filter: blur(8px);
  border-bottom-color: var(--border-subtle);
}

.header-container {
  display: flex;
  align-items: center;
  justify-content: space-between;
  max-width: 1200px;
  margin: 0 auto;
  padding: var(--space-md) var(--space-lg);
}

.logo {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  text-decoration: none;
  color: var(--text-primary);
  font-family: var(--font-display);
  font-size: 1.25rem;
  font-weight: 600;
}

.logo-mark {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--border-medium);
  border-radius: 4px;
  color: var(--accent);
  font-size: 1rem;
  font-weight: 700;
}

.logo:hover .logo-mark {
  background: var(--accent-dim);
}

.nav-desktop {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
}

.nav-link {
  position: relative;
  color: var(--text-secondary);
  text-decoration: none;
  font-size: 0.95rem;
  transition: color 0.2s ease;
}

.nav-link:hover {
  color: var(--text-primary);
}

.nav-link::after {
  content: '';
  position: absolute;
  bottom: -4px;
  left: 0;
  width: 100%;
  height: 1px;
  background: var(--accent);
  transform: scaleX(0);
  transition: transform 0.2s ease;
}

.nav-link:hover::after {
  transform: scaleX(1);
}

.github-link {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  padding: var(--space-xs) var(--space-md);
  background: var(--accent);
  border-radius: 6px;
  color: var(--text-primary);
  transition: background 0.2s ease;
}

.github-link:hover {
  background: var(--accent-bright);
}

.github-link::after {
  display: none;
}

.mobile-menu-btn {
  display: none;
  padding: var(--space-sm);
  background: transparent;
  border: none;
  color: var(--text-primary);
  cursor: pointer;
}

.close-icon {
  font-size: 1.5rem;
  line-height: 1;
}

.nav-mobile {
  display: none;
  flex-direction: column;
  padding: var(--space-md) var(--space-lg) var(--space-xl);
  border-top: 1px solid var(--border-subtle);
}

.nav-mobile .nav-link {
  padding: var(--space-md) 0;
  font-size: 1.1rem;
  border-bottom: 1px solid var(--border-subtle);
}

.nav-mobile .github-link {
  margin-top: var(--space-md);
  padding: var(--space-md);
  justify-content: center;
}

@media (max-width: 768px) {
  .nav-desktop {
    display: none;
  }

  .mobile-menu-btn {
    display: block;
  }

  .nav-mobile {
    display: flex;
  }
}

.slide-enter-active {
  animation: slideIn 0.3s ease;
}

.slide-leave-active {
  animation: slideOut 0.3s ease;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes slideOut {
  from {
    opacity: 1;
    transform: translateY(0);
  }
  to {
    opacity: 0;
    transform: translateY(-10px);
  }
}
</style>
