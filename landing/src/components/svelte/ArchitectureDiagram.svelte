<script lang="ts">
  /**
   * Architecture Diagram Component
   * Interactive ECS architecture visualization with hover descriptions
   * Shows: SYSTEMS → COMPONENTS → ENTITIES flow
   */
  import { onMount } from 'svelte';

  interface CrateNode {
    id: string;
    label: string;
    description: string;
    layer: 'system' | 'component' | 'entity';
    x: number;
    y: number;
  }

  interface Connection {
    from: string;
    to: string;
  }

  const nodes: CrateNode[] = [
    // SYSTEMS layer (top)
    {
      id: 'dsl',
      label: 'caliber-dsl',
      description: 'DSL parser → CaliberConfig. Type-safe configuration language.',
      layer: 'system',
      x: 30,
      y: 12
    },
    {
      id: 'pg',
      label: 'caliber-pg',
      description: 'pgrx extension runtime. Direct Postgres integration.',
      layer: 'system',
      x: 70,
      y: 12
    },
    // COMPONENTS layer (middle)
    {
      id: 'storage',
      label: 'storage',
      description: 'Storage trait + pgrx impl. Bypasses SQL parsing overhead.',
      layer: 'component',
      x: 15,
      y: 42
    },
    {
      id: 'context',
      label: 'context',
      description: 'Context assembly logic. Builds agent memory from hierarchy.',
      layer: 'component',
      x: 38,
      y: 42
    },
    {
      id: 'pcp',
      label: 'pcp',
      description: 'Validation, checkpoints, recovery. Harm reduction layer.',
      layer: 'component',
      x: 62,
      y: 42
    },
    {
      id: 'llm',
      label: 'llm',
      description: 'VAL (Vector Abstraction Layer). Provider-agnostic embeddings.',
      layer: 'component',
      x: 25,
      y: 62
    },
    {
      id: 'agents',
      label: 'agents',
      description: 'Multi-agent coordination. Locks, messages, delegation.',
      layer: 'component',
      x: 55,
      y: 62
    },
    // ENTITIES layer (bottom)
    {
      id: 'core',
      label: 'caliber-core',
      description: 'Entity types only. Pure data structures, no logic.',
      layer: 'entity',
      x: 50,
      y: 88
    }
  ];

  const connections: Connection[] = [
    // Systems to components
    { from: 'dsl', to: 'storage' },
    { from: 'dsl', to: 'context' },
    { from: 'pg', to: 'pcp' },
    { from: 'pg', to: 'context' },
    // Components to components
    { from: 'storage', to: 'llm' },
    { from: 'context', to: 'agents' },
    // Components to core
    { from: 'llm', to: 'core' },
    { from: 'agents', to: 'core' },
    { from: 'pcp', to: 'core' }
  ];

  let visibleNodes: string[] = $state([]);
  let hoveredNode: string | null = $state(null);

  const layerColors = {
    system: { border: '#ec4899', shadow: 'rgba(236,72,153,0.4)', text: '#ec4899' },
    component: { border: '#a855f7', shadow: 'rgba(168,85,247,0.4)', text: '#a855f7' },
    entity: { border: '#22d3ee', shadow: 'rgba(34,211,238,0.4)', text: '#22d3ee' }
  };

  const layerLabels = {
    system: 'SYSTEMS',
    component: 'COMPONENTS',
    entity: 'ENTITIES'
  };

  onMount(() => {
    // Sequential fade-in animation
    nodes.forEach((node, index) => {
      setTimeout(() => {
        visibleNodes = [...visibleNodes, node.id];
      }, 100 + index * 80);
    });
  });

  function getNodePosition(nodeId: string): CrateNode | undefined {
    return nodes.find(n => n.id === nodeId);
  }

  function getStrokeColor(fromId: string): string {
    const node = nodes.find(n => n.id === fromId);
    return node ? layerColors[node.layer].border : '#a855f7';
  }
</script>

<div class="relative w-full h-[350px] sm:h-[400px] md:h-[450px]">
  <!-- Layer labels -->
  <div class="absolute left-0 top-[8%] text-[10px] sm:text-xs font-mono text-[#ec4899] opacity-60">← SYSTEMS</div>
  <div class="absolute left-0 top-[48%] text-[10px] sm:text-xs font-mono text-[#a855f7] opacity-60">← COMPONENTS</div>
  <div class="absolute left-0 top-[85%] text-[10px] sm:text-xs font-mono text-[#22d3ee] opacity-60">← ENTITIES</div>

  <!-- SVG for connection lines -->
  <svg class="absolute inset-0 w-full h-full pointer-events-none" style="z-index: 1;">
    <defs>
      <filter id="archGlow" x="-50%" y="-50%" width="200%" height="200%">
        <feGaussianBlur stdDeviation="2" result="coloredBlur"/>
        <feMerge>
          <feMergeNode in="coloredBlur"/>
          <feMergeNode in="SourceGraphic"/>
        </feMerge>
      </filter>
    </defs>
    {#each connections as conn}
      {@const fromNode = getNodePosition(conn.from)}
      {@const toNode = getNodePosition(conn.to)}
      {#if fromNode && toNode && visibleNodes.includes(conn.from) && visibleNodes.includes(conn.to)}
        <line
          x1="{fromNode.x}%"
          y1="{fromNode.y + 4}%"
          x2="{toNode.x}%"
          y2="{toNode.y - 4}%"
          stroke={getStrokeColor(conn.from)}
          stroke-width="1.5"
          stroke-opacity="0.5"
          filter="url(#archGlow)"
          class="glow-pulse"
        />
      {/if}
    {/each}
  </svg>

  <!-- Crate nodes -->
  {#each nodes as node}
    {@const colors = layerColors[node.layer]}
    <div
      class="absolute transform -translate-x-1/2 -translate-y-1/2 transition-all duration-100"
      style="left: {node.x}%; top: {node.y}%; z-index: {hoveredNode === node.id ? 20 : 10};"
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
            class="px-2 py-1 sm:px-3 sm:py-1.5 md:px-4 md:py-2 bg-[#18181b] border-2 transition-all duration-300"
            style="
              border-color: {colors.border};
              box-shadow: 0 0 12px {colors.shadow};
              transform: scale({hoveredNode === node.id ? 1.1 : 1});
              animation: fadeInUp 0.4s ease-out forwards;
            "
          >
            <span 
              class="font-mono font-semibold text-[10px] sm:text-xs md:text-sm whitespace-nowrap"
              style="color: {colors.text};"
            >
              {node.label}
            </span>
          </div>

          <!-- Hover description panel -->
          {#if hoveredNode === node.id}
            <div
              class="absolute left-1/2 -translate-x-1/2 mt-2 w-44 sm:w-52 md:w-64 p-2 sm:p-3 glass-panel border border-[rgba(255,255,255,0.1)] text-left"
              style="animation: springIn 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) forwards; z-index: 30;"
            >
              <div class="text-[9px] sm:text-[10px] font-mono uppercase tracking-wider mb-1" style="color: {colors.text};">
                {layerLabels[node.layer]}
              </div>
              <p class="text-[10px] sm:text-xs text-[#a1a1aa] leading-relaxed">
                {node.description}
              </p>
            </div>
          {/if}
        </button>
      {/if}
    </div>
  {/each}

  <!-- Legend -->
  <div class="absolute bottom-0 left-0 right-0 flex flex-wrap justify-center gap-3 sm:gap-4 md:gap-6 text-[10px] sm:text-xs text-[#71717a] px-2">
    <div class="flex items-center gap-1.5">
      <div class="w-2 h-2 bg-[#ec4899]"></div>
      <span>Systems</span>
    </div>
    <div class="flex items-center gap-1.5">
      <div class="w-2 h-2 bg-[#a855f7]"></div>
      <span>Components</span>
    </div>
    <div class="flex items-center gap-1.5">
      <div class="w-2 h-2 bg-[#22d3ee]"></div>
      <span>Entities</span>
    </div>
  </div>
</div>

<style>
  @keyframes fadeInUp {
    from {
      opacity: 0;
      transform: translateY(15px);
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
    background: rgba(24, 24, 27, 0.9);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }

  .glow-pulse {
    animation: glowPulse 2.5s ease-in-out infinite;
  }

  @keyframes glowPulse {
    0%, 100% {
      stroke-opacity: 0.4;
    }
    50% {
      stroke-opacity: 0.7;
    }
  }
</style>
