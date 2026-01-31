<script lang="ts">
  /**
   * HexScroll - Particle network animation with hexagonal pattern
   * Floating nodes with connection lines forming a neural network effect
   * Reference: Vue HexScroll.vue
   */
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    /** Number of nodes */
    nodeCount?: number;
    /** Movement intensity multiplier */
    intensity?: number;
    /** Node density multiplier */
    density?: number;
    /** Hexagon factor for center exclusion */
    hexagonFactor?: number;
    /** Primary node color */
    primaryColor?: string;
    /** Secondary node color */
    secondaryColor?: string;
    /** Maximum connection distance */
    connectionDistance?: number;
    /** Exclude center area function */
    excludeArea?: (x: number, y: number, cx: number, cy: number, radius: number) => boolean;
    /** Additional CSS classes */
    class?: string;
  }

  let {
    nodeCount = 100,
    intensity = 1.8,
    density = 2.5,
    hexagonFactor = 1,
    primaryColor = '#60a5fa',
    secondaryColor = '#aeefff',
    connectionDistance = 180,
    excludeArea,
    class: className = '',
  }: Props = $props();

  interface Node {
    x: number;
    y: number;
    vx: number;
    vy: number;
    size: number;
    color: string;
  }

  let canvasRef: HTMLCanvasElement | null = $state(null);
  let ctx: CanvasRenderingContext2D | null = $state(null);
  let animationId: number | null = null;
  let nodes: Node[] = [];
  let width = 0;
  let height = 0;

  function resizeCanvas() {
    if (!canvasRef) return;
    // Make canvas slightly larger for edge-to-edge coverage
    width = canvasRef.width = canvasRef.offsetWidth * 1.2;
    height = canvasRef.height = canvasRef.offsetHeight * 1.2;
  }

  function initNodes() {
    nodes = [];
    const count = Math.floor(nodeCount * density);

    for (let i = 0; i < count; i++) {
      const angle = Math.random() * Math.PI * 2;
      const radius = Math.random() * Math.min(width, height) * 0.5 * hexagonFactor + 80;
      const x = width / 2 + Math.cos(angle) * radius;
      const y = height / 2 + Math.sin(angle) * radius;

      nodes.push({
        x,
        y,
        vx: (Math.random() - 0.5) * 0.7 * intensity,
        vy: (Math.random() - 0.5) * 0.7 * intensity,
        size: 2 + Math.random() * 3 * intensity,
        color: Math.random() > 0.7 ? primaryColor : secondaryColor,
      });
    }
  }

  function animate() {
    if (!ctx || !canvasRef) return;

    ctx.clearRect(0, 0, width, height);

    // Animate nodes
    for (const node of nodes) {
      node.x += node.vx;
      node.y += node.vy;

      // Bounce off edges with slight randomization
      if (node.x < 0 || node.x > width) {
        node.vx *= -1.02;
        node.vx += (Math.random() - 0.5) * 0.2;
      }
      if (node.y < 0 || node.y > height) {
        node.vy *= -1.02;
        node.vy += (Math.random() - 0.5) * 0.2;
      }

      // Exclude center area if function provided
      if (excludeArea && excludeArea(node.x, node.y, width / 2, height / 2, Math.min(width, height) * 0.35 * hexagonFactor)) {
        node.vx *= -1.05;
        node.vy *= -1.05;
        node.vx += (Math.random() - 0.5) * 0.3;
        node.vy += (Math.random() - 0.5) * 0.3;
      }
    }

    // Draw connections with gradient
    const maxDist = connectionDistance * intensity;
    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        const a = nodes[i];
        const b = nodes[j];
        const dist = Math.hypot(a.x - b.x, a.y - b.y);

        if (dist < maxDist) {
          ctx.save();
          const alpha = 0.15 + 0.35 * (1 - dist / maxDist);
          ctx.globalAlpha = alpha;

          // Create gradient for connections
          const gradient = ctx.createLinearGradient(a.x, a.y, b.x, b.y);
          gradient.addColorStop(0, a.color);
          gradient.addColorStop(1, b.color);
          ctx.strokeStyle = gradient;

          ctx.lineWidth = Math.max(0.5, 1.5 * (1 - dist / maxDist));
          ctx.beginPath();
          ctx.moveTo(a.x, a.y);
          ctx.lineTo(b.x, b.y);
          ctx.stroke();
          ctx.restore();
        }
      }
    }

    // Draw nodes with glow effect
    for (const node of nodes) {
      ctx.save();
      ctx.globalAlpha = 0.85;
      ctx.beginPath();
      ctx.arc(node.x, node.y, node.size, 0, Math.PI * 2);
      ctx.fillStyle = node.color;
      ctx.shadowColor = node.color;
      ctx.shadowBlur = 12;
      ctx.fill();

      // Add extra glow for some nodes
      if (Math.random() > 0.95) {
        ctx.globalAlpha = 0.4;
        ctx.beginPath();
        ctx.arc(node.x, node.y, node.size * 2, 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(174, 239, 255, 0.2)';
        ctx.fill();
      }

      ctx.restore();
    }

    animationId = requestAnimationFrame(animate);
  }

  function start() {
    if (!canvasRef) return;
    ctx = canvasRef.getContext('2d');
    resizeCanvas();
    initNodes();
    animate();
  }

  onMount(() => {
    start();
    window.addEventListener('resize', resizeCanvas);
  });

  onDestroy(() => {
    if (animationId !== null) {
      cancelAnimationFrame(animationId);
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('resize', resizeCanvas);
    }
  });
</script>

<div
  class="absolute inset-0 w-full h-full overflow-hidden z-10 {className}"
  role="presentation"
  aria-hidden="true"
>
  <canvas
    bind:this={canvasRef}
    class="absolute inset-0 w-full h-full pointer-events-none hex-canvas"
  ></canvas>
</div>

<style>
  .hex-canvas {
    background: transparent;
    display: block;
    filter: blur(0.5px) contrast(1.2) brightness(1.1);
    transform: scale(1.2); /* Slightly larger for full coverage */
  }
</style>
