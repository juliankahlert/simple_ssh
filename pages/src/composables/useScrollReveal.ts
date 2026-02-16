import { onMounted, onUnmounted } from 'vue';

interface ScrollRevealOptions {
  threshold?: number;
  rootMargin?: string;
}

export function useScrollReveal(selector: string, options: ScrollRevealOptions = {}): void {
  const threshold = options.threshold ?? 0.1;
  const rootMargin = options.rootMargin ?? '0px 0px -50px 0px';
  let observer: IntersectionObserver | null = null;

  onMounted(() => {
    if (typeof IntersectionObserver === 'undefined') {
      const elements = document.querySelectorAll(selector);
      elements.forEach((el) => el.classList.add('visible'));
      return;
    }

    observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add('visible');
            observer?.unobserve(entry.target);
          }
        });
      },
      { threshold, rootMargin }
    );

    const elements = document.querySelectorAll(selector);
    elements.forEach((el) => observer?.observe(el));
  });

  onUnmounted(() => {
    observer?.disconnect();
    observer = null;
  });
}