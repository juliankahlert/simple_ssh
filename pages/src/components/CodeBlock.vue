<script setup lang="ts">
import { ref, computed, onUnmounted } from 'vue';
import { CopyIcon } from './icons';
import { tokenizeRust } from '../composables/useSyntaxHighlight';

interface Props {
  label: string;
  code: string;
  language?: string;
}

const props = withDefaults(defineProps<Props>(), {
  language: 'text'
});

const copied = ref(false);
let copyTimeoutId: ReturnType<typeof setTimeout> | null = null;

async function copyToClipboard() {
  try {
    await navigator.clipboard.writeText(props.code);
    copied.value = true;
    if (copyTimeoutId) {
      clearTimeout(copyTimeoutId);
    }
    copyTimeoutId = setTimeout(() => {
      copied.value = false;
      copyTimeoutId = null;
    }, 1500);
  } catch (err) {
    console.error('Failed to copy:', err);
  }
}

onUnmounted(() => {
  if (copyTimeoutId) {
    clearTimeout(copyTimeoutId);
    copyTimeoutId = null;
  }
});

interface Token {
  type: string;
  content: string;
}

function tokenizeTOML(line: string): Token[] {
  const tokens: Token[] = [];
  let remaining = line;
  
  const patterns: { type: string; regex: RegExp }[] = [
    { type: 'comment', regex: /^(#.*$)/ },
    { type: 'string', regex: /^"(?:[^"\\]|\\.)*"/ },
    { type: 'const', regex: /^\d+/ },
    { type: 'keyword', regex: /^(true|false)\b/ },
    { type: 'punct', regex: /^[{}()\[\];:,.=]/ },
    { type: 'text', regex: /^\S+/ },
    { type: 'space', regex: /^\s+/ },
  ];
  
  while (remaining.length > 0) {
    let matched = false;
    
    for (const { type, regex } of patterns) {
      const match = remaining.match(regex);
      if (match) {
        tokens.push({ type, content: match[0] });
        remaining = remaining.slice(match[0].length);
        matched = true;
        break;
      }
    }
    
    if (!matched) {
      if (remaining.length > 0) {
        tokens.push({ type: 'text', content: remaining.charAt(0) });
        remaining = remaining.slice(1);
      }
    }
  }
  
  return tokens;
}

function tokenizePlain(line: string): Token[] {
  return [{ type: 'text', content: line }];
}

const tokenize = (line: string): Token[] => {
  if (props.language === 'toml') {
    return tokenizeTOML(line);
  }
  if (props.language === 'text' || props.language === 'rust') {
    return props.language === 'text' ? tokenizePlain(line) : tokenizeRust(line);
  }
  return tokenizePlain(line);
};

const lines = computed(() => props.code.split('\n'));
</script>

<template>
  <div class="code-block">
    <div class="header">
      <span class="label">{{ label }}</span>
      <button class="copy-btn" @click="copyToClipboard" :class="{ success: copied }">
        <CopyIcon :size="16" />
        <span v-if="copied">Copied!</span>
        <span v-else>Copy</span>
      </button>
    </div>
    <div class="code-content" :class="`language-${language}`">
      <div
        v-for="(line, i) in lines"
        :key="i"
        class="code-line"
      >
        <span
          v-for="(token, ti) in tokenize(line)"
          :key="ti"
          :class="`syn-${token.type}`"
        >{{ token.content }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.code-block {
  background: var(--bg-secondary);
  border-radius: 12px;
  overflow: hidden;
  font-family: var(--font-mono);
  border: 1px solid var(--border-light);
  transition: border-color 0.2s;
}

.code-block:hover {
  border-color: var(--border-medium);
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-sm) var(--space-md);
  background: var(--bg-tertiary);
  border-bottom: 1px solid var(--border-light);
}

.label {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.copy-btn {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  padding: var(--space-xs) var(--space-sm);
  background: transparent;
  border: 1px solid var(--border-light);
  border-radius: 4px;
  color: var(--text-muted);
  font-size: 0.75rem;
  cursor: pointer;
  transition: all 0.2s;
}

.copy-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
}

.copy-btn.success {
  background: var(--success);
  border-color: var(--success);
  color: var(--bg-primary);
  animation: copy-pulse 0.3s var(--ease-out);
}

@keyframes copy-pulse {
  50% { transform: scale(1.05); }
  100% { transform: scale(1); }
}

.code-content {
  padding: var(--space-md);
  font-size: 0.85rem;
  line-height: 1.8;
  overflow-x: auto;
}

.code-line {
  white-space: pre;
  min-width: fit-content;
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

@media (max-width: 768px) {
  .code-block {
    border-radius: 8px;
  }

  .code-content {
    font-size: 0.75rem;
    line-height: 1.6;
  }
}
</style>
