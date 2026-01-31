<script lang="ts">
  /**
   * NeuralAnimation - Canvas-based physics animation
   * Floating particles with connection lines that form neural network patterns
   * Reference: Vue NeuralAnimation.vue
   */
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    /** Particle count */
    particleCount?: number;
    /** Connection distance threshold */
    connectionDistance?: number;
    /** Particle color */
    particleColor?: string;
    /** Line color */
    lineColor?: string;
    /** Mouse interaction radius */
    mouseRadius?: number;
    /** Additional CSS classes */
    class?: string;
  }

  let {
    particleCount = 80,
    connectionDistance = 150,
    particleColor = 'rgba(79, 209, 197, 0.8)',
    lineColor = 'rgba(79, 209, 197, 0.15)',
    mouseRadius = 200,
    class: className = '',
  }: Props = $props();

  interface Particle {
    x: number;
    y: number;
    vx: number;
    vy: number;
    radius: number;
    baseRadius: number;
  }

  let canvas: HTMLCanvasElement | null = $state(null);
  let ctx: CanvasRenderingContext2D | null = $state(null);
  let particles: Particle[] = $state([]);
  let animationId: number | null = null;
  let mouse = $state({ x: -1000, y: -1000 });
  let containerRef: HTMLDivElement | null = $state(null);

  function initParticles(width: number, height: number) {
    particles = Array.from({ length: particleCount }, () => ({
      x: Math.random() * width,
      y: Math.random() * height,
      vx: (Math.random() - 0.5) * 0.5,
      vy: (Math.random() - 0.5) * 0.5,
      radius: Math.random() * 2 + 1,
      baseRadius: Math.random() * 2 + 1,
    }));
  }

  function updateParticles(width: number, height: number) {
    for (const particle of particles) {
      // Update position
      particle.x += particle.vx;
      particle.y += particle.vy;

      // Boundary bounce
      if (particle.x < 0 || particle.x > width) {
        particle.vx *= -1;
        particle.x = Math.max(0, Math.min(width, particle.x));
      }
      if (particle.y < 0 || particle.y > height) {
        particle.vy *= -1;
        particle.y = Math.max(0, Math.min(height, particle.y));
      }

      // Mouse interaction - particles are attracted/repelled
      const dx = mouse.x - particle.x;
      const dy = mouse.y - particle.y;
      const dist = Math.sqrt(dx * dx + dy * dy);

      if (dist < mouseRadius) {
        const force = (mouseRadius - dist) / mouseRadius;
        particle.vx -= (dx / dist) * force * 0.02;
        particle.vy -= (dy / dist) * force * 0.02;
        particle.radius = particle.baseRadius * (1 + force * 0.5);
      } else {
        particle.radius += (particle.baseRadius - particle.radius) * 0.1;
      }

      // Velocity damping
      particle.vx *= 0.99;
      particle.vy *= 0.99;

      // Minimum velocity
      const speed = Math.sqrt(particle.vx ** 2 + particle.vy ** 2);
      if (speed < 0.1) {
        particle.vx = (Math.random() - 0.5) * 0.3;
        particle.vy = (Math.random() - 0.5) * 0.3;
      }
    }
  }

  function drawParticles() {
    if (!ctx || !canvas) return;

    const width = canvas.width;
    const height = canvas.height;

    ctx.clearRect(0, 0, width, height);

    // Draw connections
    for (let i = 0; i < particles.length; i++) {
      for (let j = i + 1; j < particles.length; j++) {
        const dx = particles[i].x - particles[j].x;
        const dy = particles[i].y - particles[j].y;
        const dist = Math.sqrt(dx * dx + dy * dy);

        if (dist < connectionDistance) {
          const opacity = (1 - dist / connectionDistance) * 0.5;
          ctx.strokeStyle = lineColor.replace('0.15', opacity.toString());
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(particles[i].x, particles[j].y);
          ctx.lineTo(particles[j].x, particles[j].y);
          ctx.stroke();
        }
      }

      // Draw connections to mouse
      const dx = mouse.x - particles[i].x;
      const dy = mouse.y - particles[i].y;
      const dist = Math.sqrt(dx * dx + dy * dy);

      if (dist < mouseRadius) {
        const opacity = (1 - dist / mouseRadius) * 0.3;
        ctx.strokeStyle = `rgba(79, 209, 197, ${opacity})`;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(particles[i].x, particles[i].y);
        ctx.lineTo(mouse.x, mouse.y);
        ctx.stroke();
      }
    }

    // Draw particles
    for (const particle of particles) {
      ctx.fillStyle = particleColor;
      ctx.beginPath();
      ctx.arc(particle.x, particle.y, particle.radius, 0, Math.PI * 2);
      ctx.fill();

      // Glow effect
      const gradient = ctx.createRadialGradient(
        particle.x, particle.y, 0,
        particle.x, particle.y, particle.radius * 3
      );
      gradient.addColorStop(0, 'rgba(79, 209, 197, 0.3)');
      gradient.addColorStop(1, 'rgba(79, 209, 197, 0)');
      ctx.fillStyle = gradient;
      ctx.beginPath();
      ctx.arc(particle.x, particle.y, particle.radius * 3, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  function animate() {
    if (!canvas) return;
    updateParticles(canvas.width, canvas.height);
    drawParticles();
    animationId = requestAnimationFrame(animate);
  }

  function handleResize() {
    if (!canvas || !containerRef) return;
    const rect = containerRef.getBoundingClientRect();
    canvas.width = rect.width;
    canvas.height = rect.height;
    initParticles(canvas.width, canvas.height);
  }

  function handleMouseMove(e: MouseEvent) {
    if (!containerRef) return;
    const rect = containerRef.getBoundingClientRect();
    mouse = {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
    };
  }

  function handleMouseLeave() {
    mouse = { x: -1000, y: -1000 };
  }

  onMount(() => {
    if (canvas) {
      ctx = canvas.getContext('2d');
      handleResize();
      animate();

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
  onmousemove={handleMouseMove}
  onmouseleave={handleMouseLeave}
  role="presentation"
  aria-hidden="true"
>
  <canvas
    bind:this={canvas}
    class="w-full h-full"
  ></canvas>
</div>

<style>
  canvas {
    display: block;
  }
</style>
