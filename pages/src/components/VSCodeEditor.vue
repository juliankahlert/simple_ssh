<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue';
import { tokenizeRust } from '../composables/useSyntaxHighlight';

interface ExampleFile {
  name: string;
  content: string;
}

interface Props {
  examples: ExampleFile[];
  activeExample: string;
  shouldAnimate?: boolean;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  (e: 'tab-change', name: string): void;
}>();

const tabsContainer = ref<HTMLElement | null>(null);
const canScrollLeft = ref(false);
const canScrollRight = ref(false);

function updateScrollButtons() {
  if (!tabsContainer.value) return;
  const el = tabsContainer.value;
  canScrollLeft.value = el.scrollLeft > 0;
  canScrollRight.value = el.scrollLeft + el.clientWidth < el.scrollWidth;
}

function scrollTabs(direction: 'left' | 'right') {
  if (!tabsContainer.value) return;
  const scrollAmount = 150;
  tabsContainer.value.scrollBy({
    left: direction === 'left' ? -scrollAmount : scrollAmount,
    behavior: 'smooth'
  });
}

onMounted(async () => {
  await nextTick();
  requestAnimationFrame(() => {
    updateScrollButtons();
  });
  window.addEventListener('resize', updateScrollButtons);
});

onUnmounted(() => {
  window.removeEventListener('resize', updateScrollButtons);
});

const activeContent = computed(() => {
  const active = props.examples.find(e => e.name === props.activeExample);
  return active?.content || '';
});

const lines = computed(() => {
  return activeContent.value.split('\n');
});

function getLineDelay(index: number): string {
  return `${0.05 + index * 0.05}s`;
}
</script>
<template>
  <div class="editor">
    <div class="editor-header">
      <div class="editor-tabs-scroll-area">
        <button
          v-if="canScrollLeft"
          class="scroll-chevron"
          @click="scrollTabs('left')"
          aria-label="Scroll tabs left"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="15 18 9 12 15 6"/>
          </svg>
        </button>
        <div
          ref="tabsContainer"
          class="editor-tabs"
          @scroll="updateScrollButtons"
        >
          <button
            v-for="example in examples"
            :key="example.name"
            class="editor-tab"
            :class="{ active: example.name === activeExample }"
            @click="emit('tab-change', example.name)"
          >
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/>
              <polyline points="13 2 13 9 20 9"/>
            </svg>
            {{ example.name }}
          </button>
        </div>
        <button
          v-if="canScrollRight"
          class="scroll-chevron"
          @click="scrollTabs('right')"
          aria-label="Scroll tabs right"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="9 18 15 12 9 6"/>
          </svg>
        </button>
      </div>
      <div class="editor-title">
        <span>simple_ssh — examples</span>
      </div>
    </div>

    <div class="editor-pane active">
      <div class="editor-sidebar">
        <div class="sidebar-icon">
          <svg viewBox="0 0 24 24"><rect x="3" y="3" width="18" height="18" rx="2"/><line x1="9" y1="3" x2="9" y2="21"/></svg>
        </div>
        <div class="sidebar-icon">
          <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
        </div>
        <div class="sidebar-icon">
          <svg viewBox="0 0 24 24"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
        </div>
      </div>
      <div class="editor-content">
        <div class="editor-breadcrumb">
          src <span>&#8250;</span> examples <span>&#8250;</span> {{ activeExample }}
        </div>
        <div class="code-area" :key="activeExample" :class="{ animate: props.shouldAnimate }">
          <div
            v-for="(line, i) in lines"
            :key="i"
            class="code-line"
            :style="{ animationDelay: props.shouldAnimate ? getLineDelay(i) : '0s' }"
          >
            <span class="line-num">{{ i + 1 }}</span>
            <span class="line-content">
              <span
                v-for="(token, ti) in tokenizeRust(line)"
                :key="ti"
                :class="`syn-${token.type}`"
              >{{ token.content }}</span>
            </span>
          </div>
        </div>
        <div class="editor-statusbar">
          <div class="status-left">
            <span class="status-item">
              <svg viewBox="0 0 16 16" width="12" height="12">
                <path fill="currentColor" d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
              </svg>
              master*
            </span>
            <span class="status-item">Rust</span>
          </div>
          <div class="status-right">
            <span class="status-item">Ln 7, Col 9</span>
            <span class="status-item">UTF-8</span>
            <span class="status-item">LF</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.editor {
  background: var(--bg-code);
  border: 1px solid var(--border-light);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 4px 24px rgba(0,0,0,0.4), 0 1px 3px rgba(0,0,0,0.3);
  transition: transform 0.4s var(--ease-out), box-shadow 0.4s var(--ease-out);
}

.editor:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 32px rgba(0,0,0,0.45), 0 2px 4px rgba(0,0,0,0.3);
}

.editor-header {
  height: 40px;
  background: var(--bg-tertiary);
  border-bottom: 1px solid var(--border-light);
  display: flex;
  align-items: center;
  padding: 0;
  overflow: hidden;
}

.editor-tabs-scroll-area {
  display: flex;
  align-items: center;
  flex: 1;
  min-width: 0;
  height: 100%;
  overflow: hidden;
}

.editor-tabs {
  display: flex;
  height: 100%;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
  -ms-overflow-style: none;
  -webkit-overflow-scrolling: touch;
  scroll-behavior: smooth;
}

.editor-tabs::-webkit-scrollbar {
  display: none;
}

.scroll-chevron {
  width: 24px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-tertiary);
  border: none;
  cursor: pointer;
  color: var(--text-tertiary);
  transition: all 0.15s;
  flex-shrink: 0;
  z-index: 1;
}

.scroll-chevron:hover {
  background: var(--bg-elevated);
  color: var(--text-primary);
}

.scroll-chevron svg {
  width: 14px;
  height: 14px;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.editor-title {
  display: flex;
  align-items: center;
  padding: 0 var(--space-md);
  flex-shrink: 0;
  margin-left: auto;
  font-family: var(--font-mono);
  font-size: 0.7rem;
  color: var(--text-muted);
}

.editor-tab {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: 0 var(--space-md);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--text-tertiary);
  cursor: pointer;
  border: none;
  background: transparent;
  border-right: 1px solid var(--border-subtle);
  transition: all 0.15s;
  position: relative;
  height: 100%;
  white-space: nowrap;
  flex-shrink: 0;
}

.editor-tab:hover {
  background: var(--bg-elevated);
  color: var(--text-secondary);
}

.editor-tab.active {
  background: var(--bg-code);
  color: var(--text-primary);
  border-bottom: 2px solid var(--accent);
}

.editor-tab .icon {
  width: 14px;
  height: 14px;
  opacity: 0.6;
}

.editor-tab.active .icon {
  opacity: 1;
  color: var(--accent);
}

.editor-pane {
  display: flex;
  background: var(--bg-code);
  overflow: hidden;
}

.editor-sidebar {
  width: 48px;
  background: var(--bg-tertiary);
  border-right: 1px solid var(--border-light);
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--space-md) 0;
  gap: var(--space-md);
}

.sidebar-icon {
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  color: var(--text-tertiary);
  cursor: pointer;
  border-radius: 6px;
  transition: all 0.15s;
}

.sidebar-icon:hover {
  color: var(--text-primary);
  background: var(--bg-elevated);
}

.sidebar-icon svg {
  width: 20px;
  height: 20px;
  stroke: currentColor;
  fill: none;
  stroke-width: 1.5;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.editor-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.editor-breadcrumb {
  padding: var(--space-xs) var(--space-md);
  background: var(--bg-code);
  border-bottom: 1px solid var(--border-subtle);
  font-family: var(--font-mono);
  font-size: 0.7rem;
  color: var(--text-tertiary);
  display: flex;
  align-items: center;
  gap: var(--space-xs);
}

.editor-breadcrumb span {
  color: var(--text-secondary);
}

.code-area {
  padding: var(--space-md) 0;
  font-family: var(--font-mono);
  font-size: 0.8rem;
  line-height: 1.8;
  overflow-x: auto;
  overflow-y: hidden;
  min-height: 300px;
  background: var(--bg-code);
  -webkit-overflow-scrolling: touch;
  scrollbar-width: thin;
  scrollbar-color: var(--border-medium) transparent;
}

.code-area::-webkit-scrollbar {
  height: 8px;
}

.code-area::-webkit-scrollbar-track {
  background: transparent;
}

.code-area::-webkit-scrollbar-thumb {
  background: var(--border-medium);
  border-radius: 4px;
}

.code-area::-webkit-scrollbar-thumb:hover {
  background: var(--border-light);
}

.code-line {
  display: flex;
  padding: 0 var(--space-md);
  white-space: pre;
  min-width: fit-content;
  opacity: 1;
}

.code-area.animate .code-line {
  opacity: 0;
  animation: slide-in 0.3s ease-out forwards;
}

.code-line:hover {
  background: rgba(255,255,255,0.02);
}

.line-num {
  width: 40px;
  text-align: right;
  padding-right: var(--space-md);
  color: var(--text-muted);
  user-select: none;
  flex-shrink: 0;
}

.line-content {
  color: #b5b9be;
}

/* Syntax highlighting - Gotham-inspired with desaturated clrs.cc */
.syn-keyword { color: #7a6a8a; }
.syn-type { color: #5a8a9a; }
.syn-func { color: #9a9a7a; }
.syn-string { color: #9a8a6a; }
.syn-comment { color: #5a7a5a; }
.syn-macro { color: #5a8a9a; }
.syn-punct { color: #9a9a9a; }
.syn-path { color: #6a7a9a; }
.syn-const { color: #7a9a7a; }
.syn-text { color: #b5b9be; }
.syn-space { color: #b5b9be; }

@keyframes slide-in {
  from { opacity: 0; transform: translateX(-10px); }
  to { opacity: 1; transform: translateX(0); }
}

.editor-statusbar {
  height: 24px;
  background: linear-gradient(90deg, var(--accent), var(--accent-bright));
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--space-md);
  font-family: var(--font-mono);
  font-size: 0.7rem;
  color: var(--bg-primary);
}

.status-left, .status-right {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
}

.status-item {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
}

.status-item svg {
  width: 12px;
  height: 12px;
  fill: currentColor;
}

@media (max-width: 768px) {
  .editor-section {
    padding: var(--space-xl) var(--space-md);
    max-width: 100%;
  }

  .editor {
    border-radius: 8px;
  }

  .editor-header {
    overflow: hidden;
  }

  .editor-tabs {
    flex-shrink: 0;
    min-width: 0;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }

  .editor-tabs::-webkit-scrollbar {
    display: none;
  }

  .editor-tab {
    padding: 0 var(--space-sm);
    font-size: 0.7rem;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .editor-sidebar {
    display: none;
  }

  .code-area {
    font-size: 0.75rem;
    line-height: 1.6;
    overflow-x: auto;
    overflow-y: hidden;
    -webkit-overflow-scrolling: touch;
  }

  .line-num {
    width: 32px;
    padding-right: var(--space-sm);
  }
}

@media (max-width: 480px) {
  .editor-tabs {
    gap: 0;
  }

  .editor-tab {
    padding: 0 var(--space-xs);
    font-size: 0.65rem;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .editor-tab .icon {
    width: 12px;
    height: 12px;
  }
}
</style>
