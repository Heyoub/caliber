<script lang="ts">
  /**
   * GridAnimation - Animated grid background with perspective
   * Creates a retro/cyberpunk grid effect that pulses and moves
   * Reference: Vue GridBackground.vue
   */
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    /** Grid line color */
    lineColor?: string;
    /** Grid line opacity */
    lineOpacity?: number;
    /** Grid cell size in pixels */
    cellSize?: number;
    /** Perspective depth */
    perspective?: number;
    /** Animation speed (lower = slower) */
    speed?: number;
    /** Glow color */
    glowColor?: string;
    /** Enable pulse animation */
    pulse?: boolean;
    /** Additional CSS classes */
    class?: string;
  }

  let {
    lineColor = 'rgba(79, 209, 197, 0.3)',
    lineOpacity = 0.3,
    cellSize = 50,
    perspective = 500,
    speed = 0.5,
    glowColor = 'rgba(79, 209, 197, 0.5)',
    pulse = true,
    class: className = '',
  }: Props = $props();

  let canvas: HTMLCanvasElement | null = $state(null);
  let ctx: CanvasRenderingContext2D | null = $state(null);
  let animationId: number | null = null;
  let time = 0;
  let containerRef: HTMLDivElement | null = $state(null);

  function draw() {
    if (!ctx || !canvas) return;

    const width = canvas.width;
    const height = canvas.height;
    const centerX = width / 2;
    const horizonY = height * 0.4;

    ctx.clearRect(0, 0, width, height);

    // Calculate pulse factor
    const pulseFactor = pulse ? 0.8 + Math.sin(time * 0.5) * 0.2 : 1;

    // Draw horizontal lines (with perspective)
    const numHorizontalLines = 20;
    for (let i = 0; i <= numHorizontalLines; i++) {
      const progress = i / numHorizontalLines;
      const y = horizonY + (height - horizonY) * Math.pow(progress, 1.5);

      // Lines get thinner and more transparent as they go up
      const alpha = lineOpacity * progress * pulseFactor;
      const lineWidth = 1 + progress * 2;

      ctx.strokeStyle = lineColor.replace('0.3', alpha.toString());
      ctx.lineWidth = lineWidth;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    // Draw vertical lines (converging to horizon)
    const numVerticalLines = 15;
    for (let i = -numVerticalLines; i <= numVerticalLines; i++) {
      const xBottom = centerX + (i * cellSize * 2);
      const xTop = centerX + (i * cellSize * 0.1);

      const alpha = lineOpacity * (1 - Math.abs(i) / numVerticalLines) * pulseFactor;

      ctx.strokeStyle = lineColor.replace('0.3', alpha.toString());
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(xTop, horizonY);
      ctx.lineTo(xBottom, height);
      ctx.stroke();
    }

    // Draw horizon glow
    const gradient = ctx.createLinearGradient(0, horizonY - 50, 0, horizonY + 50);
    gradient.addColorStop(0, 'transparent');
    gradient.addColorStop(0.5, glowColor.replace('0.5', (0.3 * pulseFactor).toString()));
    gradient.addColorStop(1, 'transparent');

    ctx.fillStyle = gradient;
    ctx.fillRect(0, horizonY - 50, width, 100);

    // Animate time
    time += speed * 0.016; // Approximate 60fps

    animationId = requestAnimationFrame(draw);
  }

  function handleResize() {
    if (!canvas || !containerRef) return;
    const rect = containerRef.getBoundingClientRect();
    canvas.width = rect.width;
    canvas.height = rect.height;
  }

  onMount(() => {
    if (canvas) {
      ctx = canvas.getContext('2d');
      handleResize();
      draw();

      window.addEventListener('resize', handleResize);
    }
  });

  onDestroy(() => {
    if (animationId !== null) {
      cancelAnimationFrame(animationId);
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('resize', handleResize);
    }
  });
</script>

<div
  bind:this={containerRef}
  class="absolute inset-0 overflow-hidden {className}"
  role="presentation"
  aria-hidden="true"
>
  <canvas
    bind:this={canvas}
    class="w-full h-full"
    style="perspective: {perspective}px;"
  ></canvas>
</div>

<style>
  canvas {
    display: block;
  }
</style>
