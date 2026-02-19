<template>
  <section class="hero">
    <div class="hero-badge">
      <span class="dot"></span>
      Async SSH library for Rust
    </div>
    <h1>SSH made <span class="accent">simple</span><span class="cursor" aria-hidden="true">|</span>.</h1>
    <p class="hero-sub">
      A lightweight, asynchronous Rust library for SSH operations. Execute remote commands, transfer files via SCP, and manage interactive PTY sessions with an ergonomic builder API. Built on <code>tokio</code> and <code>russh</code>.
    </p>
    <div class="hero-actions">
      <a href="#install" class="btn btn-primary" style="animation-delay: 0.3s">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
        Get Started
      </a>
      <a href="#examples" class="btn btn-secondary" style="animation-delay: 0.45s">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
        View Examples
      </a>
    </div>
  </section>
</template>

<style scoped>
.hero {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  padding: var(--space-2xl) var(--space-xl);
  max-width: 1200px;
  margin: 0 auto;
  position: relative;
}

.hero::after {
  content: '';
  position: absolute;
  top: 40%;
  left: 20%;
  width: 600px;
  height: 600px;
  background: radial-gradient(circle, rgba(90, 138, 154, 0.06) 0%, transparent 70%);
  pointer-events: none;
  z-index: -1;
}

.hero-badge {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-xs) var(--space-md);
  background: var(--bg-secondary);
  border: 1px solid var(--border-light);
  border-radius: 100px;
  font-size: 0.8rem;
  color: var(--text-tertiary);
  margin-bottom: var(--space-lg);
  width: fit-content;
  animation: fade-up 0.8s var(--ease-out) both;
  box-shadow: 0 0 12px var(--accent-glow);
}

.hero-badge .dot {
  width: 6px;
  height: 6px;
  background: var(--accent);
  border-radius: 50%;
  animation: pulse 2s ease-in-out infinite;
  box-shadow: 0 0 8px var(--accent);
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(0.8); }
}

@keyframes fade-up {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.hero h1 {
  font-size: clamp(3rem, 8vw, 5.5rem);
  font-weight: 700;
  color: var(--text-primary);
  line-height: 1.05;
  letter-spacing: -0.03em;
  margin-bottom: var(--space-md);
  animation: fade-up 0.8s var(--ease-out) 0.1s both;
}

.hero h1 .accent {
  background: linear-gradient(135deg, var(--accent) 0%, var(--accent-bright) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.hero h1 .accent::selection {
  -webkit-text-fill-color: var(--bg-primary);
  background: var(--accent);
}

.hero h1 .cursor {
  color: var(--accent);
  animation: blink-cursor 1s step-end infinite;
  font-weight: 300;
  margin-left: -0.05em;
}

@keyframes blink-cursor {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.hero-sub {
  font-size: 1.35rem;
  color: var(--text-secondary);
  max-width: 600px;
  margin-bottom: var(--space-xl);
  line-height: 1.7;
  animation: fade-up 0.8s var(--ease-out) 0.2s both;
}

.hero-sub code {
  font-family: var(--font-mono);
  font-size: 0.85em;
  background: var(--bg-tertiary);
  padding: 0.15em 0.4em;
  border-radius: 4px;
  color: var(--accent);
  border: 1px solid var(--border-light);
}

.hero-actions {
  display: flex;
  gap: var(--space-md);
  flex-wrap: wrap;
}

.hero-actions .btn {
  animation: fade-up 0.8s var(--ease-out) both;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-md) var(--space-lg);
  border-radius: 8px;
  font-family: var(--font-display);
  font-size: 0.95rem;
  font-weight: 600;
  text-decoration: none;
  border: none;
  cursor: pointer;
  transition: all 0.2s var(--ease-out);
}

.btn-primary {
  background: var(--accent);
  color: var(--bg-primary);
}

.btn-primary:hover {
  background: var(--accent-bright);
  transform: translateY(-2px);
}

.btn-secondary {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-medium);
}

.btn-secondary:hover {
  border-color: var(--accent);
  color: var(--accent);
  transform: translateY(-2px);
}

@media (max-width: 768px) {
  .hero {
    padding: calc(var(--space-2xl) + 56px) var(--space-md) var(--space-xl);
    min-height: auto;
  }

  .hero h1 {
    font-size: clamp(2rem, 8vw, 3rem);
    line-height: 1.1;
  }

  .hero-sub {
    font-size: 1rem;
    line-height: 1.6;
  }

  .hero-actions {
    flex-direction: column;
    gap: var(--space-sm);
  }

  .btn {
    width: 100%;
    justify-content: center;
    padding: var(--space-sm) var(--space-md);
  }
}

@media (max-width: 480px) {
  .hero h1 {
    font-size: clamp(1.75rem, 7vw, 2.5rem);
  }
}
</style>
