<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import CodeBlock from './CodeBlock.vue';

const sectionRef = ref<HTMLElement | null>(null);
const isVisible = ref(false);
let observer: IntersectionObserver | null = null;

const cargoTomlCode = `[dependencies]
simple_ssh = { version = "0.1.3", features = ["cli"] }

[profile.release]
opt-level = "s"
lto = true
strip = true`;

const ptyExampleCode = `use simple_ssh::Session;
use simple_ssh::PtyHandle;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::builder()
        .connect("user@localhost", 22)
        .await?;

    let mut pty = PtyHandle::new(session)
        .with_program("/bin/sh")
        .spawn()
        .await?;

    let output = pty.run_command("ls -la")
        .await?;

    println!("{}", output);
    Ok(())
}`;

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
  if (observer) {
    observer.disconnect();
    observer = null;
  }
});
</script>

<template>
  <section class="code-examples-section" id="examples" ref="sectionRef" :class="{ 'visible': isVisible }">
    <div class="section-header">
      <span class="section-label">Examples</span>
      <h2 class="section-title">Copy, paste, run.</h2>
      <p class="section-desc">
        Practical examples for common SSH operations.
      </p>
    </div>

    <div class="code-blocks">
      <CodeBlock
        label="Cargo.toml"
        :code="cargoTomlCode"
        language="toml"
      />

      <CodeBlock
        label="Programmatic PTY"
        :code="ptyExampleCode"
        language="rust"
      />
    </div>
  </section>
</template>

<style scoped>
.code-examples-section {
  padding: var(--space-xl) var(--space-xl);
  max-width: 900px;
  margin: 0 auto;
  opacity: 0;
  transform: translateY(20px);
  transition: opacity 0.6s ease, transform 0.6s ease;
}

.code-examples-section.visible {
  opacity: 1;
  transform: translateY(0);
}

.section-header {
  text-align: center;
  margin-bottom: var(--space-lg);
}

.section-desc {
  margin: 0 auto;
}

.code-blocks {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

@media (max-width: 768px) {
  .code-examples-section {
    padding: var(--space-lg) var(--space-md);
  }

  .section-title {
    font-size: 1.5rem;
  }

  .code-blocks {
    gap: var(--space-sm);
  }
}
</style>
