<script lang="ts">
  /**
   * IcosahedronAnimation - Three.js 3D rotating icosahedron
   * Reference: Vue IcosahedronAnimation.vue
   *
   * Note: This component requires three.js to be installed.
   * In a production setup, this would use dynamic imports for code splitting.
   */
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    /** Size of the container */
    size?: number;
    /** Rotation speed */
    rotationSpeed?: number;
    /** Primary color (hex) */
    primaryColor?: string;
    /** Secondary color (hex) */
    secondaryColor?: string;
    /** Enable wireframe mode */
    wireframe?: boolean;
    /** Enable vertex coloring */
    vertexColors?: boolean;
    /** Enable mouse interaction */
    interactive?: boolean;
    /** Additional CSS classes */
    class?: string;
  }

  let {
    size = 300,
    rotationSpeed = 0.002,
    primaryColor = '#4fd1c5',
    secondaryColor = '#a855f7',
    wireframe = true,
    vertexColors = true,
    interactive = true,
    class: className = '',
  }: Props = $props();

  let containerRef: HTMLDivElement | null = $state(null);
  let animationId: number | null = null;
  let isThreeLoaded = $state(false);
  let hasError = $state(false);

  // Three.js references (initialized after load)
  let scene: any;
  let camera: any;
  let renderer: any;
  let mesh: any;
  let mouseX = 0;
  let mouseY = 0;

  async function initThree() {
    if (typeof window === 'undefined') return;

    try {
      // Dynamic import for code splitting
      const THREE = await import('three');

      if (!containerRef) return;

      // Scene setup
      scene = new THREE.Scene();

      // Camera setup
      camera = new THREE.PerspectiveCamera(75, 1, 0.1, 1000);
      camera.position.z = 2;

      // Renderer setup
      renderer = new THREE.WebGLRenderer({
        antialias: true,
        alpha: true,
      });
      renderer.setSize(size, size);
      renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
      containerRef.appendChild(renderer.domElement);

      // Create icosahedron geometry
      const geometry = new THREE.IcosahedronGeometry(1, 0);

      // Apply vertex colors if enabled
      if (vertexColors) {
        const count = geometry.attributes.position.count;
        const colors = new Float32Array(count * 3);

        const color1 = new THREE.Color(primaryColor);
        const color2 = new THREE.Color(secondaryColor);

        for (let i = 0; i < count; i++) {
          const t = i / count;
          const color = color1.clone().lerp(color2, Math.sin(t * Math.PI));
          colors[i * 3] = color.r;
          colors[i * 3 + 1] = color.g;
          colors[i * 3 + 2] = color.b;
        }

        geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
      }

      // Material setup
      const material = wireframe
        ? new THREE.MeshBasicMaterial({
            wireframe: true,
            vertexColors: vertexColors,
            color: vertexColors ? undefined : primaryColor,
            transparent: true,
            opacity: 0.8,
          })
        : new THREE.MeshStandardMaterial({
            vertexColors: vertexColors,
            color: vertexColors ? undefined : primaryColor,
            metalness: 0.3,
            roughness: 0.4,
          });

      // Create mesh
      mesh = new THREE.Mesh(geometry, material);
      scene.add(mesh);

      // Add ambient light for non-wireframe mode
      if (!wireframe) {
        const ambientLight = new THREE.AmbientLight(0xffffff, 0.5);
        scene.add(ambientLight);

        const pointLight = new THREE.PointLight(primaryColor, 1);
        pointLight.position.set(5, 5, 5);
        scene.add(pointLight);
      }

      isThreeLoaded = true;
      animate();
    } catch (error) {
      console.warn('Three.js not available:', error);
      hasError = true;
    }
  }

  function animate() {
    if (!mesh || !renderer || !scene || !camera) return;

    // Base rotation
    mesh.rotation.x += rotationSpeed;
    mesh.rotation.y += rotationSpeed * 1.5;

    // Mouse interaction
    if (interactive) {
      mesh.rotation.x += (mouseY * 0.001 - mesh.rotation.x * 0.1) * 0.1;
      mesh.rotation.y += (mouseX * 0.001 - mesh.rotation.y * 0.1) * 0.1;
    }

    renderer.render(scene, camera);
    animationId = requestAnimationFrame(animate);
  }

  function handleMouseMove(e: MouseEvent) {
    if (!interactive || !containerRef) return;

    const rect = containerRef.getBoundingClientRect();
    const centerX = rect.width / 2;
    const centerY = rect.height / 2;

    mouseX = (e.clientX - rect.left - centerX) * 0.5;
    mouseY = (e.clientY - rect.top - centerY) * 0.5;
  }

  function handleMouseLeave() {
    mouseX = 0;
    mouseY = 0;
  }

  function handleResize() {
    if (!renderer || !camera || !containerRef) return;

    const newSize = Math.min(containerRef.offsetWidth, containerRef.offsetHeight);
    renderer.setSize(newSize, newSize);
  }

  onMount(() => {
    initThree();

    if (interactive) {
      window.addEventListener('resize', handleResize);
    }
  });

  onDestroy(() => {
    if (animationId !== null) {
      cancelAnimationFrame(animationId);
    }

    if (renderer && containerRef) {
      containerRef.removeChild(renderer.domElement);
      renderer.dispose();
    }

    if (typeof window !== 'undefined') {
      window.removeEventListener('resize', handleResize);
    }
  });
</script>

<div
  bind:this={containerRef}
  class="relative flex items-center justify-center {className}"
  style="width: {size}px; height: {size}px;"
  onmousemove={handleMouseMove}
  onmouseleave={handleMouseLeave}
  role="presentation"
  aria-hidden="true"
>
  {#if !isThreeLoaded && !hasError}
    <!-- Loading fallback -->
    <div class="absolute inset-0 flex items-center justify-center">
      <div class="w-16 h-16 border-2 border-teal-500/30 border-t-teal-500 rounded-full animate-spin"></div>
    </div>
  {/if}

  {#if hasError}
    <!-- Error fallback - CSS-only animated shape -->
    <div class="icosahedron-fallback">
      <div class="icosahedron-shape"></div>
    </div>
  {/if}
</div>

<style>
  /* CSS-only fallback animation */
  .icosahedron-fallback {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    perspective: 1000px;
  }

  .icosahedron-shape {
    width: 100px;
    height: 100px;
    position: relative;
    transform-style: preserve-3d;
    animation: rotate-3d 10s linear infinite;
  }

  .icosahedron-shape::before,
  .icosahedron-shape::after {
    content: '';
    position: absolute;
    inset: 0;
    border: 2px solid;
    border-color: hsl(var(--teal-500) / 0.6);
    clip-path: polygon(50% 0%, 100% 38%, 82% 100%, 18% 100%, 0% 38%);
  }

  .icosahedron-shape::after {
    transform: rotateX(180deg);
    border-color: hsl(var(--purple-500) / 0.6);
  }

  @keyframes rotate-3d {
    0% {
      transform: rotateX(0deg) rotateY(0deg);
    }
    100% {
      transform: rotateX(360deg) rotateY(360deg);
    }
  }
</style>
