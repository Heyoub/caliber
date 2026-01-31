<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Size, ColorToken } from '../types';

  /**
   * Icon - SVG icon wrapper with dynamic loading
   */
  interface Props {
    /** Icon name from registry */
    name?: string;
    /** Size variant */
    size?: Size;
    /** Color token */
    color?: ColorToken;
    /** Spin animation */
    spin?: boolean;
    /** Pulse animation */
    pulse?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Inline SVG content slot */
    children?: Snippet;
  }

  let {
    name,
    size = 'md',
    color,
    spin = false,
    pulse = false,
    class: className = '',
    children,
  }: Props = $props();

  // Size configurations
  const sizeConfigs: Record<Size, string> = {
    xs: 'w-3 h-3',
    sm: 'w-4 h-4',
    md: 'w-5 h-5',
    lg: 'w-6 h-6',
    xl: 'w-8 h-8',
    '2xl': 'w-10 h-10',
  };

  // Color configurations
  const colorConfigs: Record<string, string> = {
    teal: 'text-[hsl(var(--teal-400))]',
    coral: 'text-[hsl(var(--coral-400))]',
    purple: 'text-[hsl(var(--purple-400))]',
    pink: 'text-[hsl(var(--pink-400))]',
    mint: 'text-[hsl(var(--mint-400))]',
    amber: 'text-[hsl(var(--amber-400))]',
    slate: 'text-[hsl(var(--slate-400))]',
    ghost: 'text-current',
  };

  // Common icons registry (inline SVG paths)
  const iconRegistry: Record<string, string> = {
    // Navigation
    'chevron-down': 'M6 9l6 6 6-6',
    'chevron-up': 'M18 15l-6-6-6 6',
    'chevron-left': 'M15 18l-6-6 6-6',
    'chevron-right': 'M9 6l6 6-6 6',
    'arrow-left': 'M19 12H5m7-7l-7 7 7 7',
    'arrow-right': 'M5 12h14m-7-7l7 7-7 7',

    // Actions
    'plus': 'M12 5v14m-7-7h14',
    'minus': 'M5 12h14',
    'x': 'M6 18L18 6M6 6l12 12',
    'check': 'M5 13l4 4L19 7',
    'edit': 'M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z',
    'trash': 'M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16',
    'copy': 'M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z',
    'save': 'M17 21v-8H7v8M7 3v5h8V3M5 21h14a2 2 0 002-2V7.414a1 1 0 00-.293-.707l-4.414-4.414A1 1 0 0015.586 2H5a2 2 0 00-2 2v15a2 2 0 002 2z',

    // UI Elements
    'menu': 'M4 6h16M4 12h16M4 18h16',
    'search': 'M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z',
    'settings': 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z',
    'user': 'M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z',
    'bell': 'M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9',

    // Status
    'info': 'M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z',
    'warning': 'M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z',
    'error': 'M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z',
    'success': 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z',

    // Files
    'file': 'M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z',
    'folder': 'M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z',
    'document': 'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z',

    // AI/Tool
    'sparkles': 'M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z',
    'cpu': 'M9 3v2m6-2v2M9 19v2m6-2v2M3 9h2m-2 6h2m14-6h2m-2 6h2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z',
    'terminal': 'M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z',
    'code': 'M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4',

    // Memory/Data
    'database': 'M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4',
    'layers': 'M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z',
    'git-branch': 'M6 3v12M18 9a3 3 0 100-6 3 3 0 000 6zM6 21a3 3 0 100-6 3 3 0 000 6zm12-3a9 9 0 00-9-9',
  };

  // Derived values
  let sizeClass = $derived(sizeConfigs[size]);
  let colorClass = $derived(color ? (colorConfigs[color.split('-')[0]] || colorConfigs.ghost) : '');

  // Icon path
  let iconPath = $derived(name ? iconRegistry[name] : undefined);

  // Computed classes
  let computedClasses = $derived([
    'inline-block',
    'flex-shrink-0',
    sizeClass,
    colorClass,
    spin ? 'animate-spin' : '',
    pulse ? 'animate-pulse' : '',
    className,
  ].filter(Boolean).join(' '));
</script>

{#if children}
  <span class={computedClasses}>
    {@render children()}
  </span>
{:else if iconPath}
  <svg
    class={computedClasses}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d={iconPath} />
  </svg>
{:else}
  <!-- Fallback placeholder -->
  <span class="{computedClasses} bg-current opacity-20 rounded"></span>
{/if}
