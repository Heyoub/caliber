<!--
  MemoryGraph.svelte - Interactive memory hierarchy visualization
  From MemoryHierarchy.svelte concept

  Features:
  - Parallax scrolling effect
  - Hover tooltips
  - Connection lines with glow
  - Interactive nodes
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { GraphNode, ColorPalette, CMSContent } from '../types/index.js';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Graph nodes */
    nodes: GraphNode[];
    /** Currently selected node ID */
    selectedId?: string;
    /** Show connection lines */
    showConnections?: boolean;
    /** Enable parallax effect */
    parallax?: boolean;
    /** Zoom level (0.5 - 2) */
    zoom?: number;
    /** Additional CSS classes */
    class?: string;
    /** Node renderer slot */
    nodeRenderer?: Snippet<[{ node: GraphNode; selected: boolean }]>;
    /** Tooltip slot */
    tooltipRenderer?: Snippet<[{ node: GraphNode }]>;
    /** Event handlers */
    onNodeSelect?: (node: GraphNode) => void;
    onNodeHover?: (node: GraphNode | null) => void;
  }

  let {
    cms = {},
    nodes,
    selectedId,
    showConnections = true,
    parallax = true,
    zoom = 1,
    class: className = '',
    nodeRenderer,
    tooltipRenderer,
    onNodeSelect,
    onNodeHover
  }: Props = $props();

  // State
  let containerRef: HTMLDivElement | undefined = $state();
  let hoveredNode: GraphNode | null = $state(null);
  let mouseX = $state(0);
  let mouseY = $state(0);
  let scrollY = $state(0);

  // Color map for nodes
  const colorStyles: Record<ColorPalette, { bg: string; border: string; glow: string }> = {
    teal: {
      bg: 'bg-teal-500/20',
      border: 'border-teal-500/50',
      glow: 'shadow-[0_0_20px_hsl(var(--teal-500)/0.4)]'
    },
    coral: {
      bg: 'bg-coral-500/20',
      border: 'border-coral-500/50',
      glow: 'shadow-[0_0_20px_hsl(var(--coral-500)/0.4)]'
    },
    purple: {
      bg: 'bg-purple-500/20',
      border: 'border-purple-500/50',
      glow: 'shadow-[0_0_20px_hsl(var(--purple-500)/0.4)]'
    },
    pink: {
      bg: 'bg-pink-500/20',
      border: 'border-pink-500/50',
      glow: 'shadow-[0_0_20px_hsl(var(--pink-500)/0.4)]'
    },
    mint: {
      bg: 'bg-mint-500/20',
      border: 'border-mint-500/50',
      glow: 'shadow-[0_0_20px_hsl(var(--mint-500)/0.4)]'
    },
    amber: {
      bg: 'bg-amber-500/20',
      border: 'border-amber-500/50',
      glow: 'shadow-[0_0_20px_hsl(var(--amber-500)/0.4)]'
    },
    slate: {
      bg: 'bg-slate-500/20',
      border: 'border-slate-500/50',
      glow: 'shadow-[0_0_20px_hsl(var(--slate-500)/0.4)]'
    },
    ghost: {
      bg: 'bg-white/10',
      border: 'border-white/20',
      glow: ''
    }
  };

  // Type icons
  const typeIcons: Record<GraphNode['type'], string> = {
    trajectory: 'M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z',
    scope: 'M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z',
    turn: 'M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z'
  };

  // Get node by ID
  function getNode(id: string): GraphNode | undefined {
    return nodes.find(n => n.id === id);
  }

  // Mouse move handler for parallax
  function handleMouseMove(e: MouseEvent) {
    if (!containerRef) return;

    const rect = containerRef.getBoundingClientRect();
    mouseX = (e.clientX - rect.left - rect.width / 2) / rect.width;
    mouseY = (e.clientY - rect.top - rect.height / 2) / rect.height;
  }

  // Scroll handler for parallax
  function handleScroll() {
    scrollY = window.scrollY;
  }

  // Calculate parallax offset
  function getParallaxOffset(node: GraphNode, factor: number = 1): string {
    if (!parallax) return '';

    const offsetX = mouseX * 20 * factor;
    const offsetY = mouseY * 20 * factor;

    return `translate(${offsetX}px, ${offsetY}px)`;
  }

  // Node click handler
  function handleNodeClick(node: GraphNode) {
    onNodeSelect?.(node);
  }

  // Node hover handlers
  function handleNodeEnter(node: GraphNode) {
    hoveredNode = node;
    onNodeHover?.(node);
  }

  function handleNodeLeave() {
    hoveredNode = null;
    onNodeHover?.(null);
  }

  // Generate connection path between two nodes
  function getConnectionPath(from: GraphNode, to: GraphNode): string {
    const x1 = from.x;
    const y1 = from.y;
    const x2 = to.x;
    const y2 = to.y;

    // Bezier curve for smooth connection
    const midX = (x1 + x2) / 2;
    const midY = (y1 + y2) / 2;
    const cp1x = midX;
    const cp1y = y1;
    const cp2x = midX;
    const cp2y = y2;

    return `M ${x1} ${y1} C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${x2} ${y2}`;
  }

  // Check if node is selected
  function isSelected(nodeId: string): boolean {
    return selectedId === nodeId;
  }

  // Reactive setup for scroll listener
  $effect(() => {
    if (typeof window !== 'undefined') {
      window.addEventListener('scroll', handleScroll);
      return () => window.removeEventListener('scroll', handleScroll);
    }
  });
</script>

<div
  bind:this={containerRef}
  class={`memory-graph relative overflow-hidden bg-slate-950 rounded-2xl border border-slate-800 ${className}`}
  onmousemove={handleMouseMove}
  style="transform: scale({zoom})"
>
  <!-- Background gradient -->
  <div class="absolute inset-0 bg-gradient-to-br from-purple-500/5 via-transparent to-teal-500/5 pointer-events-none"></div>

  <!-- Grid pattern background -->
  <div class="absolute inset-0 opacity-10 pointer-events-none" style="
    background-image:
      linear-gradient(hsl(var(--slate-700)) 1px, transparent 1px),
      linear-gradient(90deg, hsl(var(--slate-700)) 1px, transparent 1px);
    background-size: 50px 50px;
  "></div>

  <!-- SVG layer for connections -->
  {#if showConnections}
    <svg class="absolute inset-0 w-full h-full pointer-events-none" style="min-height: 400px;">
      <defs>
        <!-- Glow filter -->
        <filter id="connection-glow" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="2" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <!-- Connection lines -->
      {#each nodes as node (node.id)}
        {#each node.connections as connectionId}
          {@const targetNode = getNode(connectionId)}
          {#if targetNode}
            <path
              d={getConnectionPath(node, targetNode)}
              fill="none"
              stroke="url(#connection-gradient-{node.color})"
              stroke-width="2"
              opacity="0.4"
              filter="url(#connection-glow)"
              class="transition-opacity duration-300"
              class:opacity-70={isSelected(node.id) || isSelected(connectionId)}
            />
          {/if}
        {/each}
      {/each}

      <!-- Gradient definitions for connections -->
      <defs>
        <linearGradient id="connection-gradient-teal" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="hsl(var(--teal-500))" />
          <stop offset="100%" stop-color="hsl(var(--teal-400))" stop-opacity="0.3" />
        </linearGradient>
        <linearGradient id="connection-gradient-purple" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="hsl(var(--purple-500))" />
          <stop offset="100%" stop-color="hsl(var(--purple-400))" stop-opacity="0.3" />
        </linearGradient>
        <linearGradient id="connection-gradient-pink" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="hsl(var(--pink-500))" />
          <stop offset="100%" stop-color="hsl(var(--pink-400))" stop-opacity="0.3" />
        </linearGradient>
      </defs>
    </svg>
  {/if}

  <!-- Nodes layer -->
  <div class="relative" style="min-height: 400px;">
    {#each nodes as node (node.id)}
      {@const styles = colorStyles[node.color] || colorStyles.teal}
      {@const selected = isSelected(node.id)}
      {@const hovered = hoveredNode?.id === node.id}
      {@const parallaxFactor = node.type === 'trajectory' ? 0.5 : node.type === 'scope' ? 0.75 : 1}

      <button
        class={`
          absolute flex flex-col items-center gap-2 p-4 rounded-xl
          border border-solid backdrop-blur-sm cursor-pointer
          transition-all duration-300 group
          ${styles.bg} ${styles.border}
          ${selected ? styles.glow : ''}
          ${hovered ? 'scale-110' : ''}
        `}
        style="
          left: {node.x}px;
          top: {node.y}px;
          transform: translate(-50%, -50%) {getParallaxOffset(node, parallaxFactor)};
        "
        onclick={() => handleNodeClick(node)}
        onmouseenter={() => handleNodeEnter(node)}
        onmouseleave={handleNodeLeave}
      >
        {#if nodeRenderer}
          {@render nodeRenderer({ node, selected })}
        {:else}
          <!-- Node icon -->
          <div class={`w-10 h-10 rounded-lg ${styles.bg} flex items-center justify-center`}>
            <svg
              class="w-5 h-5"
              class:text-teal-400={node.color === 'teal'}
              class:text-purple-400={node.color === 'purple'}
              class:text-pink-400={node.color === 'pink'}
              class:text-mint-400={node.color === 'mint'}
              class:text-coral-400={node.color === 'coral'}
              class:text-amber-400={node.color === 'amber'}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={typeIcons[node.type]} />
            </svg>
          </div>

          <!-- Node label -->
          <span class="text-xs font-medium text-slate-200 whitespace-nowrap">
            {node.label}
          </span>

          <!-- Connection count badge -->
          {#if node.connections.length > 0}
            <span class="absolute -top-1 -right-1 w-5 h-5 text-[10px] font-bold bg-slate-800 text-slate-300 rounded-full flex items-center justify-center border border-slate-700">
              {node.connections.length}
            </span>
          {/if}
        {/if}
      </button>
    {/each}
  </div>

  <!-- Tooltip -->
  {#if hoveredNode && tooltipRenderer}
    <div
      class="absolute z-50 pointer-events-none"
      style="left: {hoveredNode.x + 60}px; top: {hoveredNode.y - 20}px;"
    >
      {@render tooltipRenderer({ node: hoveredNode })}
    </div>
  {:else if hoveredNode}
    <div
      class="absolute z-50 px-3 py-2 bg-slate-800/95 backdrop-blur-sm rounded-lg border border-slate-700 shadow-xl pointer-events-none text-sm"
      style="left: {hoveredNode.x + 60}px; top: {hoveredNode.y - 20}px; transform: translateY(-50%);"
    >
      <div class="font-medium text-slate-100">{hoveredNode.label}</div>
      <div class="text-xs text-slate-400 capitalize">{hoveredNode.type}</div>
      {#if hoveredNode.metadata}
        <div class="mt-1 pt-1 border-t border-slate-700 text-xs text-slate-500">
          {#each Object.entries(hoveredNode.metadata) as [key, value]}
            <div>{key}: {value}</div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Legend -->
  <div class="absolute bottom-4 left-4 flex items-center gap-4 text-xs text-slate-400">
    <div class="flex items-center gap-1.5">
      <div class="w-3 h-3 rounded bg-teal-500/30 border border-teal-500/50"></div>
      <span>{cms.trajectoryLabel || 'Trajectory'}</span>
    </div>
    <div class="flex items-center gap-1.5">
      <div class="w-3 h-3 rounded bg-purple-500/30 border border-purple-500/50"></div>
      <span>{cms.scopeLabel || 'Scope'}</span>
    </div>
    <div class="flex items-center gap-1.5">
      <div class="w-3 h-3 rounded bg-pink-500/30 border border-pink-500/50"></div>
      <span>{cms.turnLabel || 'Turn'}</span>
    </div>
  </div>

  <!-- Zoom controls -->
  <div class="absolute bottom-4 right-4 flex items-center gap-1">
    <span class="text-xs text-slate-500 mr-2">{Math.round(zoom * 100)}%</span>
  </div>
</div>

<style>
  /* Node hover glow animation */
  button:hover {
    z-index: 10;
  }

  /* Connection pulse animation */
  @keyframes connection-pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.7; }
  }

  path.pulsing {
    animation: connection-pulse 2s ease-in-out infinite;
  }
</style>
