<script lang="ts">
  /**
   * ParallaxHero - Multi-layer parallax scrolling hero section
   * Provides depth effect with multiple scroll-responsive layers
   * Reference: Vue ParallaxHero.vue, HeroSection.vue
   */
  import { onMount, onDestroy } from 'svelte';
  import type { Snippet } from 'svelte';

  interface ParallaxLayer {
    id: string;
    speed: number; // 0 = static, 1 = full scroll speed
    content?: Snippet;
    class?: string;
  }

  interface Props {
    /** Array of parallax layers */
    layers?: ParallaxLayer[];
    /** Base height of hero section */
    height?: string;
    /** Enable mouse parallax effect */
    mouseParallax?: boolean;
    /** Mouse parallax intensity */
    mouseIntensity?: number;
    /** Background gradient */
    gradient?: string;
    /** Additional CSS classes */
    class?: string;
    /** Children content */
    children?: Snippet;
  }

  let {
    layers = [],
    height = '100vh',
    mouseParallax = true,
    mouseIntensity = 0.02,
    gradient = 'from-[#0a0a0f] via-[#0f0f1a] to-[#0a0a0f]',
    class: className = '',
    children,
  }: Props = $props();

  let containerRef: HTMLDivElement | null = $state(null);
  let scrollY = $state(0);
  let mouseX = $state(0);
  let mouseY = $state(0);
  let viewportHeight = $state(0);
  let isInView = $state(false);

  function handleScroll() {
    if (!containerRef) return;

    const rect = containerRef.getBoundingClientRect();
    const containerTop = rect.top;
    const containerHeight = rect.height;

    // Check if container is in view
    isInView = containerTop < viewportHeight && containerTop + containerHeight > 0;

    if (isInView) {
      // Calculate scroll progress relative to container position
      scrollY = -containerTop;
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (!mouseParallax || !containerRef) return;

    const rect = containerRef.getBoundingClientRect();
    const centerX = rect.width / 2;
    const centerY = rect.height / 2;

    mouseX = (e.clientX - rect.left - centerX) * mouseIntensity;
    mouseY = (e.clientY - rect.top - centerY) * mouseIntensity;
  }

  function handleResize() {
    viewportHeight = window.innerHeight;
  }

  onMount(() => {
    viewportHeight = window.innerHeight;
    handleScroll();

    window.addEventListener('scroll', handleScroll, { passive: true });
    window.addEventListener('resize', handleResize, { passive: true });
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('scroll', handleScroll);
      window.removeEventListener('resize', handleResize);
    }
  });

  // Compute layer transforms
  function getLayerStyle(layer: ParallaxLayer): string {
    const scrollOffset = scrollY * layer.speed;
    const mouseOffsetX = mouseX * (1 - layer.speed);
    const mouseOffsetY = mouseY * (1 - layer.speed);

    return `transform: translate3d(${mouseOffsetX}px, ${scrollOffset + mouseOffsetY}px, 0);`;
  }
</script>

<div
  bind:this={containerRef}
  class="relative overflow-hidden bg-gradient-to-b {gradient} {className}"
  style="height: {height};"
  onmousemove={handleMouseMove}
  role="banner"
>
  <!-- Parallax layers -->
  {#each layers as layer (layer.id)}
    <div
      class="absolute inset-0 will-change-transform {layer.class || ''}"
      style={getLayerStyle(layer)}
    >
      {#if layer.content}
        {@render layer.content()}
      {/if}
    </div>
  {/each}

  <!-- Main content layer (no parallax) -->
  <div class="relative z-10 h-full">
    {#if children}
      {@render children()}
    {/if}
  </div>

  <!-- Gradient overlays -->
  <div class="absolute inset-0 bg-gradient-to-t from-[#0a0a0f] via-transparent to-transparent pointer-events-none"></div>
  <div class="absolute inset-0 bg-gradient-to-b from-[#0a0a0f]/50 via-transparent to-transparent pointer-events-none"></div>
</div>

<style>
  /* Hardware acceleration for smooth parallax */
  .will-change-transform {
    will-change: transform;
    backface-visibility: hidden;
    perspective: 1000px;
  }
</style>
