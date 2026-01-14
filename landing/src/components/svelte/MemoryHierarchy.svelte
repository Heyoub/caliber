<script lang="ts">
  /**
   * Memory Hierarchy Visualization Component
   * Animated diagram showing Trajectory → Scope → Turn/Artifact → Note
   * Requirements: 1.1, 1.2, 1.5, 5.6
   */
  import { onMount, onDestroy } from 'svelte';
  import { spring } from 'motion';

  interface MemoryNode {
    id: string;
    label: string;
    description: string;
    depth: number;
    color: string;
    x: number;
    y: number;
  }

  interface Connection {
    from: string;
    to: string;
  }

  const nodes: MemoryNode[] = [
    {
      id: 'trajectory',
      label: 'Trajectory',
      description: 'Task container - the complete journey of an agent task',
      depth: 0,
      color: 'neon-cyan',
      x: 50,
      y: 15
    },
    {
      id: 'scope',
      label: 'Scope',
      description: 'Context partition - isolated memory boundaries within a task',
      depth: 1,
      color: 'neon-purple',
      x: 30,
      y: 40
    },
    {
      id: 'turn',
      label: 'Turn',
      description: 'Ephemeral buffer - temporary conversation state',
      depth: 2,
      color: 'neon-pink',
      x: 15,
      y: 70
    },
    {
      id: 'artifact',
      label: 'Artifact',
      description: 'Preserved output - permanent results and generated content',
      depth: 2,
      color: 'neon-pink',
      x: 45,
      y: 70
    },
    {
      id: 'note',
      label: 'Note',
      description: 'Cross-trajectory knowledge - shared insights across tasks',
      depth: 1,
      color: 'rust-accent',
      x: 75,
      y: 40
    }
  ];

  const connections: Connection[] = [
    { from: 'trajectory', to: 'scope' },
    { from: 'trajectory', to: 'note' },
    { from: 'scope', to: 'turn' },
    { from: 'scope', to: 'artifact' }
  ];

  let visibleNodes: string[] = $state([]);
  let hoveredNode: string | null = $state(null);
  let mounted = $state(false);
  let containerRef: HTMLDivElement | null = $state(null);
  
  // Parallax offset state - nodes shift based on scroll position and depth
  let parallaxOffsets: Record<string, { x: number; y: number }> = $state({});
  
  // Spring animations for smooth parallax movement
  const springConfigs = nodes.reduce((acc, node) => {
    acc[node.id] = { x: spring(0, { stiffness: 100, damping: 20 }), y: spring(0, { stiffness: 100, damping: 20 }) };
    return acc;
  }, {} as Record<string, { x: ReturnType<typeof spring>; y: ReturnType<typeof spring> }>);

  // Parallax multipliers based on depth (deeper = more movement)
  const depthMultipliers: Record<number, number> = {
    0: 0.3,  // Trajectory - subtle movement
    1: 0.6,  // Scope, Note - medium movement
    2: 1.0   // Turn, Artifact - most movement
  };

  function handleScroll() {
    if (!containerRef) return;
    
    const rect = containerRef.getBoundingClientRect();
    const viewportHeight = window.innerHeight;
    
    // Calculate how far the container is from center of viewport
    const containerCenter = rect.top + rect.height / 2;
    const viewportCenter = viewportHeight / 2;
    const scrollProgress = (containerCenter - viewportCenter) / viewportHeight;
    
    // Update spring targets for each node based on depth
    nodes.forEach(node => {
      const multiplier = depthMultipliers[node.depth] || 0.5;
      const offsetY = scrollProgress * 30 * multiplier;
      const offsetX = scrollProgress * 10 * multiplier * (node.x > 50 ? 1 : -1);
      
      springConfigs[node.id].x.set(offsetX);
      springConfigs[node.id].y.set(offsetY);
    });
  }

  // Subscribe to spring values and update parallaxOffsets
  $effect(() => {
    if (!mounted) return;
    
    const unsubscribers: (() => void)[] = [];
    
    nodes.forEach(node => {
      const unsubX = springConfigs[node.id].x.on('change', (latest: number) => {
        parallaxOffsets = {
          ...parallaxOffsets,
          [node.id]: { 
            x: latest, 
            y: parallaxOffsets[node.id]?.y || 0 
          }
        };
      });
      
      const unsubY = springConfigs[node.id].y.on('change', (latest: number) => {
        parallaxOffsets = {
          ...parallaxOffsets,
          [node.id]: { 
            x: parallaxOffsets[node.id]?.x || 0, 
            y: latest 
          }
        };
      });
      
      unsubscribers.push(unsubX, unsubY);
    });
    
    return () => {
      unsubscribers.forEach(unsub => unsub());
    };
  });

  onMount(() => {
    mounted = true;
    
    // Initialize parallax offsets
    nodes.forEach(node => {
      parallaxOffsets[node.id] = { x: 0, y: 0 };
    });
    
    // Sequential fade-in animation
    nodes.forEach((node, index) => {
      setTimeout(() => {
        visibleNodes = [...visibleNodes, node.id];
      }, 200 + index * 150);
    });
    
    // Add scroll listener for parallax
    window.addEventListener('scroll', handleScroll, { passive: true });
    handleScroll(); // Initial calculation
  });
  
  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('scroll', handleScroll);
    }
  });

  function getNodePosition(nodeId: string): MemoryNode | undefined {
    return nodes.find(n => n.id === nodeId);
  }

  function getColorClass(color: string): string {
    const colorMap: Record<string, string> = {
      'neon-cyan': 'border-[#22d3ee] shadow-[0_0_15px_rgba(34,211,238,0.4)]',
      'neon-purple': 'border-[#a855f7] shadow-[0_0_15px_rgba(168,85,247,0.4)]',
      'neon-pink': 'border-[#ec4899] shadow-[0_0_15px_rgba(236,72,153,0.4)]',
      'rust-accent': 'border-[#f59e0b] shadow-[0_0_15px_rgba(245,158,11,0.4)]'
    };
    return colorMap[color] || '';
  }

  function getTextColorClass(color: string): string {
    const colorMap: Record<string, string> = {
      'neon-cyan': 'text-[#22d3ee]',
      'neon-purple': 'text-[#a855f7]',
      'neon-pink': 'text-[#ec4899]',
      'rust-accent': 'text-[#f59e0b]'
    };
    return colorMap[color] || '';
  }

  function getStrokeColor(fromId: string): string {
    const node = nodes.find(n => n.id === fromId);
    const colorMap: Record<string, string> = {
      'neon-cyan': '#22d3ee',
      'neon-purple': '#a855f7',
      'neon-pink': '#ec4899',
      'rust-accent': '#f59e0b'
    };
    return node ? colorMap[node.color] || '#22d3ee' : '#22d3ee';
  }
</script>

<div class="relative w-full h-[320px] sm:h-[380px] md:h-[450px]" bind:this={containerRef}>
  <!-- SVG for connection lines -->
  <svg class="absolute inset-0 w-full h-full pointer-events-none" style="z-index: 1;">
    <defs>
      <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
        <feGaussianBlur stdDeviation="3" result="coloredBlur"/>
        <feMerge>
          <feMergeNode in="coloredBlur"/>
          <feMergeNode in="SourceGraphic"/>
        </feMerge>
      </filter>
    </defs>
    {#each connections as conn}
      {@const fromNode = getNodePosition(conn.from)}
      {@const toNode = getNodePosition(conn.to)}
      {@const fromOffset = parallaxOffsets[conn.from] || { x: 0, y: 0 }}
      {@const toOffset = parallaxOffsets[conn.to] || { x: 0, y: 0 }}
      {#if fromNode && toNode && visibleNodes.includes(conn.from) && visibleNodes.includes(conn.to)}
        <line
          x1="calc({fromNode.x}% + {fromOffset.x}px)"
          y1="calc({fromNode.y + 5}% + {fromOffset.y}px)"
          x2="calc({toNode.x}% + {toOffset.x}px)"
          y2="calc({toNode.y - 5}% + {toOffset.y}px)"
          stroke={getStrokeColor(conn.from)}
          stroke-width="2"
          stroke-opacity="0.6"
          filter="url(#glow)"
          class="glow-pulse"
        />
      {/if}
    {/each}
  </svg>

  <!-- Memory nodes -->
  {#each nodes as node}
    {@const offset = parallaxOffsets[node.id] || { x: 0, y: 0 }}
    <div
      class="absolute transform -translate-x-1/2 -translate-y-1/2 transition-all duration-100 ease-out"
      style="left: calc({node.x}% + {offset.x}px); top: calc({node.y}% + {offset.y}px); z-index: {hoveredNode === node.id ? 20 : 10};"
    >
      {#if visibleNodes.includes(node.id)}
        <button
          type="button"
          class="relative group cursor-pointer"
          onmouseenter={() => hoveredNode = node.id}
          onmouseleave={() => hoveredNode = null}
          onfocus={() => hoveredNode = node.id}
          onblur={() => hoveredNode = null}
        >
          <!-- Node box -->
          <div
            class="px-3 py-1.5 sm:px-4 sm:py-2 md:px-6 md:py-3 bg-[#18181b] border-2 transition-all duration-300 {getColorClass(node.color)} {hoveredNode === node.id ? 'scale-110' : 'scale-100'}"
            style="animation: fadeInUp 0.5s ease-out forwards;"
          >
            <span class="font-title font-bold text-xs sm:text-sm md:text-base {getTextColorClass(node.color)}">
              {node.label}
            </span>
          </div>

          <!-- Hover description panel -->
          {#if hoveredNode === node.id}
            <div
              class="absolute left-1/2 -translate-x-1/2 mt-2 w-40 sm:w-48 md:w-64 p-2 sm:p-3 md:p-4 glass-panel border border-[rgba(255,255,255,0.1)] text-left"
              style="animation: springIn 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) forwards; z-index: 30;"
            >
              <p class="text-[10px] sm:text-xs md:text-sm text-[#a1a1aa] leading-relaxed">
                {node.description}
              </p>
            </div>
          {/if}
        </button>
      {/if}
    </div>
  {/each}

  <!-- Legend -->
  <div class="absolute bottom-0 left-0 right-0 flex flex-wrap justify-center gap-2 sm:gap-3 md:gap-6 text-[10px] sm:text-xs text-[#71717a] px-2">
    <div class="flex items-center gap-1 sm:gap-1.5">
      <div class="w-1.5 h-1.5 sm:w-2 sm:h-2 bg-[#22d3ee]"></div>
      <span>Meta</span>
    </div>
    <div class="flex items-center gap-1 sm:gap-1.5">
      <div class="w-1.5 h-1.5 sm:w-2 sm:h-2 bg-[#a855f7]"></div>
      <span>Working</span>
    </div>
    <div class="flex items-center gap-1 sm:gap-1.5">
      <div class="w-1.5 h-1.5 sm:w-2 sm:h-2 bg-[#ec4899]"></div>
      <span>Ephemeral</span>
    </div>
    <div class="flex items-center gap-1 sm:gap-1.5">
      <div class="w-1.5 h-1.5 sm:w-2 sm:h-2 bg-[#f59e0b]"></div>
      <span>Semantic</span>
    </div>
  </div>
</div>

<style>
  @keyframes fadeInUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes springIn {
    0% {
      opacity: 0;
      transform: translateX(-50%) scale(0.95) translateY(-5px);
    }
    60% {
      transform: translateX(-50%) scale(1.02) translateY(2px);
    }
    100% {
      opacity: 1;
      transform: translateX(-50%) scale(1) translateY(0);
    }
  }

  .glass-panel {
    background: rgba(24, 24, 27, 0.85);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }

  .glow-pulse {
    animation: glowPulse 2s ease-in-out infinite;
  }

  @keyframes glowPulse {
    0%, 100% {
      stroke-opacity: 0.6;
    }
    50% {
      stroke-opacity: 0.9;
    }
  }
</style>
