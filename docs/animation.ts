import gsap from 'gsap';
import ScrollTrigger from 'gsap/ScrollTrigger';

if (typeof window !== 'undefined') {
  gsap.registerPlugin(ScrollTrigger);
}

export function animateElement(selector: string | HTMLElement, options: Record<string, any>): void {
  if (typeof window !== 'undefined') {
    gsap.to(selector, options);
  }
}

export function createParallaxEffect(selector: string | HTMLElement, options: Record<string, any> = {}): void {
  if (typeof window !== 'undefined') {
    gsap.to(selector, {
      y: options.y || 100,
      ease: options.ease || "none",
      scrollTrigger: {
        trigger: selector,
        start: options.start || "top bottom",
        end: options.end || "bottom top",
        scrub: options.scrub ?? true,
      }
    });
  }
}

export function initSmoothScroll(): void {
  if (typeof document !== 'undefined') {
    document.documentElement.style.scrollBehavior = "smooth";
  }
}
