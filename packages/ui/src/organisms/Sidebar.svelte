<!--
  Sidebar.svelte - Navigation sidebar organism
  Ported from SidebarNav.vue pattern

  Features:
  - Collapsible sections
  - Footer section
  - Nested menu structure
  - Laser/spotlight effect on hover
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { CMSContent } from '../types/index.js';

  interface NavItem {
    id: string;
    label: string;
    href?: string;
    icon?: string;
    badge?: string | number;
    children?: NavItem[];
    active?: boolean;
  }

  interface NavSection {
    id: string;
    title?: string;
    items: NavItem[];
    collapsible?: boolean;
  }

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Navigation sections */
    sections: NavSection[];
    /** Collapsed state */
    collapsed?: boolean;
    /** Show laser effect on hover */
    laserEffect?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Header slot */
    header?: Snippet;
    /** Footer slot */
    footer?: Snippet;
    /** Custom nav item renderer */
    itemRenderer?: Snippet<[{ item: NavItem; depth: number; collapsed: boolean }]>;
    /** Event handlers */
    onNavigate?: (item: NavItem) => void;
    onToggleCollapse?: () => void;
    onSectionToggle?: (sectionId: string) => void;
  }

  let {
    cms = {},
    sections,
    collapsed = false,
    laserEffect = true,
    class: className = '',
    header,
    footer,
    itemRenderer,
    onNavigate,
    onToggleCollapse,
    onSectionToggle
  }: Props = $props();

  // State
  let sidebarRef: HTMLElement | undefined = $state();
  let laserX = $state(0);
  let laserY = $state(0);
  let isHovering = $state(false);
  let collapsedSections = $state<string[]>([]);

  // Toggle section collapse
  function toggleSection(sectionId: string) {
    if (collapsedSections.includes(sectionId)) {
      collapsedSections = collapsedSections.filter(id => id !== sectionId);
    } else {
      collapsedSections = [...collapsedSections, sectionId];
    }
    onSectionToggle?.(sectionId);
  }

  // Check if section is collapsed
  function isSectionCollapsed(sectionId: string): boolean {
    return collapsedSections.includes(sectionId);
  }

  // Laser effect mouse tracking
  function handleMouseMove(e: MouseEvent) {
    if (!laserEffect || !sidebarRef) return;

    const rect = sidebarRef.getBoundingClientRect();
    laserX = e.clientX - rect.left;
    laserY = e.clientY - rect.top;
  }

  function handleMouseEnter() {
    if (laserEffect) isHovering = true;
  }

  function handleMouseLeave() {
    if (laserEffect) isHovering = false;
  }

  // Handle navigation item click
  function handleItemClick(item: NavItem) {
    onNavigate?.(item);
  }
</script>

<aside
  bind:this={sidebarRef}
  class={`
    flex flex-col h-full bg-slate-900 border-r border-slate-800 transition-all duration-300
    ${collapsed ? 'w-16' : 'w-64'}
    ${className}
  `}
  onmousemove={handleMouseMove}
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
>
  <!-- Laser effect overlay -->
  {#if laserEffect && isHovering}
    <div
      class="absolute inset-0 pointer-events-none transition-opacity duration-300"
      style="
        background: radial-gradient(
          circle at {laserX}px {laserY}px,
          hsl(var(--teal-500) / 0.1) 0%,
          transparent 40%
        );
      "
    ></div>
  {/if}

  <!-- Header -->
  <div class="flex-shrink-0 border-b border-slate-800">
    {#if header}
      {@render header()}
    {:else}
      <div class="flex items-center justify-between px-4 py-4">
        {#if !collapsed}
          <span class="text-lg font-semibold text-slate-100">
            {cms.title || 'Navigation'}
          </span>
        {/if}

        <!-- Collapse toggle -->
        <button
          class="p-2 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition-colors"
          onclick={() => onToggleCollapse?.()}
          title={collapsed ? (cms.expandLabel || 'Expand') : (cms.collapseLabel || 'Collapse')}
        >
          <svg
            class="w-5 h-5 transition-transform duration-300"
            class:rotate-180={collapsed}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7" />
          </svg>
        </button>
      </div>
    {/if}
  </div>

  <!-- Navigation sections -->
  <nav class="flex-1 overflow-y-auto py-2">
    {#each sections as section, sectionIndex (section.id)}
      {@const isCollapsed = isSectionCollapsed(section.id)}

      <!-- Section divider (not first) -->
      {#if sectionIndex > 0}
        <hr class="my-2 mx-4 border-slate-800" />
      {/if}

      <!-- Section header -->
      {#if section.title && !collapsed}
        <button
          class="flex items-center justify-between w-full px-4 py-2 text-xs font-semibold text-slate-500 uppercase tracking-wider hover:text-slate-400 transition-colors"
          onclick={() => section.collapsible && toggleSection(section.id)}
        >
          <span>{section.title}</span>
          {#if section.collapsible}
            <svg
              class="w-3 h-3 transition-transform duration-200"
              class:-rotate-90={isCollapsed}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          {/if}
        </button>
      {/if}

      <!-- Section items -->
      {#if !isCollapsed || !section.collapsible}
        <div class="space-y-0.5">
          {#each section.items as item (item.id)}
            {#if itemRenderer}
              {@render itemRenderer({ item, depth: 0, collapsed })}
            {:else}
              <a
                href={item.href || '#'}
                class={`
                  flex items-center gap-3 px-4 py-2.5 text-sm font-medium rounded-lg mx-2 transition-all relative group
                  ${item.active
                    ? 'bg-teal-500/20 text-teal-100'
                    : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/50'
                  }
                `}
                onclick={(e) => { e.preventDefault(); handleItemClick(item); }}
              >
                <!-- Icon placeholder -->
                {#if item.icon}
                  <span class="w-5 h-5 flex items-center justify-center flex-shrink-0">
                    {item.icon}
                  </span>
                {:else}
                  <span class="w-5 h-5 flex items-center justify-center flex-shrink-0 rounded bg-slate-800">
                    <span class="text-xs">{item.label[0]}</span>
                  </span>
                {/if}

                <!-- Label (hidden when collapsed) -->
                {#if !collapsed}
                  <span class="flex-1 truncate">{item.label}</span>

                  <!-- Badge -->
                  {#if item.badge}
                    <span class="px-2 py-0.5 text-xs font-medium bg-slate-800 text-slate-400 rounded-full">
                      {item.badge}
                    </span>
                  {/if}

                  <!-- Active indicator line -->
                  {#if item.active}
                    <span class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-6 bg-teal-500 rounded-r"></span>
                  {/if}
                {/if}

                <!-- Hover underline animation -->
                <span
                  class="absolute bottom-0 left-1/2 right-1/2 h-px bg-teal-500 opacity-0 group-hover:left-4 group-hover:right-4 group-hover:opacity-100 transition-all duration-300"
                ></span>
              </a>

              <!-- Children (nested items) -->
              {#if item.children && item.children.length > 0 && !collapsed}
                <div class="ml-6 pl-4 border-l border-slate-800 space-y-0.5">
                  {#each item.children as child (child.id)}
                    <a
                      href={child.href || '#'}
                      class={`
                        flex items-center gap-2 px-3 py-2 text-sm rounded-lg transition-colors
                        ${child.active
                          ? 'text-teal-300 bg-teal-500/10'
                          : 'text-slate-500 hover:text-slate-300 hover:bg-slate-800/30'
                        }
                      `}
                      onclick={(e) => { e.preventDefault(); handleItemClick(child); }}
                    >
                      <span class="truncate">{child.label}</span>
                    </a>
                  {/each}
                </div>
              {/if}
            {/if}
          {/each}
        </div>
      {/if}
    {/each}
  </nav>

  <!-- Footer -->
  <div class="flex-shrink-0 mt-auto border-t border-slate-800">
    {#if footer}
      {@render footer()}
    {:else if !collapsed}
      <div class="p-4 space-y-3">
        <!-- About card -->
        <div class="px-3 py-3 rounded-lg bg-slate-800/50 border border-slate-700/50">
          <h3 class="text-xs font-semibold mb-1 text-teal-400">
            {cms.aboutTitle || 'About'}
          </h3>
          <p class="text-xs text-slate-400">
            {cms.aboutText || 'CALIBER Pack Editor'}
          </p>
        </div>

        <!-- Contact/legal links -->
        <div class="flex justify-center gap-4 text-xs text-slate-500">
          <a href="#" class="hover:text-teal-400 transition-colors">
            {cms.privacyLabel || 'Privacy'}
          </a>
          <a href="#" class="hover:text-teal-400 transition-colors">
            {cms.termsLabel || 'Terms'}
          </a>
          <a href="#" class="hover:text-teal-400 transition-colors">
            {cms.helpLabel || 'Help'}
          </a>
        </div>

        <!-- Copyright -->
        <div class="text-center text-xs text-slate-600">
          {cms.copyright || `\u00A9 ${new Date().getFullYear()} CALIBER`}
        </div>
      </div>
    {/if}
  </div>
</aside>

<style>
  /* Smooth scrollbar */
  nav::-webkit-scrollbar {
    width: 4px;
  }

  nav::-webkit-scrollbar-track {
    background: transparent;
  }

  nav::-webkit-scrollbar-thumb {
    background: hsl(var(--slate-700));
    border-radius: 2px;
  }

  nav::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--slate-600));
  }

  /* Expand animation for nested items */
  div[class*="ml-6"] {
    animation: expand 0.2s ease-out;
  }

  @keyframes expand {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
