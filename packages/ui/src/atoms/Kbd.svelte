<script lang="ts">
  import type { Size } from '../types';

  /**
   * Kbd - Keyboard shortcut display
   */
  interface Props {
    /** Single key or array of keys */
    keys?: string | string[];
    /** Size variant */
    size?: Size;
    /** Additional CSS classes */
    class?: string;
  }

  let {
    keys = [],
    size = 'sm',
    class: className = '',
  }: Props = $props();

  // Normalize keys to array
  let keyArray = $derived(Array.isArray(keys) ? keys : [keys]);

  // Key symbol mapping
  const keySymbols: Record<string, string> = {
    // Modifiers
    'cmd': '\u2318',
    'command': '\u2318',
    'ctrl': '\u2303',
    'control': '\u2303',
    'alt': '\u2325',
    'option': '\u2325',
    'shift': '\u21E7',
    'meta': '\u2318',

    // Navigation
    'enter': '\u21B5',
    'return': '\u21B5',
    'tab': '\u21E5',
    'escape': 'Esc',
    'esc': 'Esc',
    'backspace': '\u232B',
    'delete': '\u2326',
    'space': '\u2423',

    // Arrows
    'up': '\u2191',
    'down': '\u2193',
    'left': '\u2190',
    'right': '\u2192',
    'arrowup': '\u2191',
    'arrowdown': '\u2193',
    'arrowleft': '\u2190',
    'arrowright': '\u2192',

    // Other
    'home': '\u2196',
    'end': '\u2198',
    'pageup': '\u21DE',
    'pagedown': '\u21DF',
  };

  // Size configurations
  const sizeConfigs: Record<Size, string> = {
    xs: 'text-[10px] px-1 py-0.5 min-w-[16px]',
    sm: 'text-xs px-1.5 py-0.5 min-w-[20px]',
    md: 'text-sm px-2 py-1 min-w-[24px]',
    lg: 'text-base px-2.5 py-1 min-w-[28px]',
    xl: 'text-lg px-3 py-1.5 min-w-[32px]',
    '2xl': 'text-xl px-4 py-2 min-w-[40px]',
  };

  // Format key for display
  function formatKey(key: string): string {
    const lower = key.toLowerCase();
    return keySymbols[lower] || key.toUpperCase();
  }

  let sizeClass = $derived(sizeConfigs[size]);
</script>

<span class="inline-flex items-center gap-0.5 {className}">
  {#each keyArray as key, i}
    {#if i > 0}
      <span class="text-[hsl(var(--slate-600))] text-xs mx-0.5">+</span>
    {/if}
    <kbd
      class="inline-flex items-center justify-center
             font-mono font-medium
             text-[hsl(var(--slate-300))]
             bg-[hsl(var(--slate-800))]
             border border-[hsl(var(--slate-700))]
             border-b-2 border-b-[hsl(var(--slate-600))]
             rounded
             shadow-sm
             {sizeClass}"
    >
      {formatKey(key)}
    </kbd>
  {/each}
</span>
