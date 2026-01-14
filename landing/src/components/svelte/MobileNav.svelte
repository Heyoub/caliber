<script lang="ts">
  /**
   * Mobile Navigation Component
   * Hamburger menu with slide-in glass effect panel
   * Requirements: 8.3
   */
  import { onMount, onDestroy } from 'svelte';

  interface NavLink {
    href: string;
    label: string;
    external?: boolean;
  }

  const navLinks: NavLink[] = [
    { href: '#problems', label: 'Problem' },
    { href: '#solutions', label: 'Solution' },
    { href: '#architecture', label: 'Architecture' },
    { href: '#pricing', label: 'Pricing' },
    { href: 'https://github.com/heyoub/caliber', label: 'GitHub', external: true },
  ];

  let isOpen = $state(false);
  let menuRef: HTMLDivElement | null = $state(null);

  function toggleMenu() {
    isOpen = !isOpen;
    // Prevent body scroll when menu is open
    if (typeof document !== 'undefined') {
      document.body.style.overflow = isOpen ? 'hidden' : '';
    }
  }

  function closeMenu() {
    isOpen = false;
    if (typeof document !== 'undefined') {
      document.body.style.overflow = '';
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isOpen) {
      closeMenu();
    }
  }

  function handleClickOutside(event: MouseEvent) {
    if (menuRef && !menuRef.contains(event.target as Node) && isOpen) {
      closeMenu();
    }
  }

  onMount(() => {
    if (typeof window !== 'undefined') {
      window.addEventListener('keydown', handleKeydown);
      window.addEventListener('click', handleClickOutside);
    }
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', handleKeydown);
      window.removeEventListener('click', handleClickOutside);
      document.body.style.overflow = '';
    }
  });
</script>

<div class="md:hidden" bind:this={menuRef}>
  <!-- Hamburger Button -->
  <button
    type="button"
    onclick={toggleMenu}
    class="relative z-50 p-2 text-[#a1a1aa] hover:text-[#fafafa] border border-[#27272a] transition-colors"
    aria-label={isOpen ? 'Close menu' : 'Open menu'}
    aria-expanded={isOpen}
  >
    <div class="w-6 h-5 flex flex-col justify-between">
      <span 
        class="block h-0.5 w-full bg-current transition-all duration-300 origin-center"
        class:rotate-45={isOpen}
        class:translate-y-2={isOpen}
      ></span>
      <span 
        class="block h-0.5 w-full bg-current transition-all duration-300"
        class:opacity-0={isOpen}
        class:scale-x-0={isOpen}
      ></span>
      <span 
        class="block h-0.5 w-full bg-current transition-all duration-300 origin-center"
        class:-rotate-45={isOpen}
        class:-translate-y-2={isOpen}
      ></span>
    </div>
  </button>

  <!-- Backdrop -->
  {#if isOpen}
    <div 
      class="fixed inset-0 bg-black/60 backdrop-blur-sm z-40"
      style="animation: fadeIn 0.2s ease-out forwards;"
      onclick={closeMenu}
      aria-hidden="true"
    ></div>
  {/if}

  <!-- Slide-in Menu -->
  <nav
    class="fixed top-0 right-0 h-full w-72 max-w-[80vw] z-50 glass-panel border-l border-[rgba(255,255,255,0.1)] transform transition-transform duration-300 ease-out"
    class:translate-x-0={isOpen}
    class:translate-x-full={!isOpen}
    aria-hidden={!isOpen}
    data-mobile-menu
  >
    <div class="flex flex-col h-full pt-20 pb-8 px-6">
      <!-- Navigation Links -->
      <div class="flex-1 space-y-2">
        {#each navLinks as link}
          <a
            href={link.href}
            onclick={closeMenu}
            class="flex items-center justify-between px-4 py-4 text-base font-medium text-[#a1a1aa] hover:text-[#fafafa] hover:bg-white/5 border border-[#27272a] transition-all"
            target={link.external ? '_blank' : undefined}
            rel={link.external ? 'noopener noreferrer' : undefined}
          >
            <span>{link.label}</span>
            {#if link.external}
              <svg class="w-4 h-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
              </svg>
            {/if}
          </a>
        {/each}
      </div>

      <!-- CTA Button -->
      <div class="mt-6">
        <a
          href="#pricing"
          onclick={closeMenu}
          class="flex items-center justify-center gap-2 w-full px-6 py-4 bg-[#a855f7]/10 border-2 border-[#a855f7] text-[#fafafa] font-semibold transition-all hover:bg-[#a855f7]/20"
          style="box-shadow: 0 0 10px rgba(168, 85, 247, 0.3);"
        >
          <span>Try CALIBER Cloud</span>
        </a>
      </div>

      <!-- Footer info -->
      <div class="mt-6 pt-6 border-t border-[#27272a]">
        <p class="text-xs text-[#71717a] text-center">
          Open source under AGPL-3.0
        </p>
      </div>
    </div>
  </nav>
</div>

<style>
  .glass-panel {
    background: rgba(24, 24, 27, 0.95);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
