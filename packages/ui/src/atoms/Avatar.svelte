<script lang="ts">
  import type { Size, ColorToken } from '../types';

  /**
   * Avatar - User avatar with initials fallback
   */
  interface Props {
    /** Image source URL */
    src?: string;
    /** Alt text for image */
    alt?: string;
    /** User name for initials fallback */
    name?: string;
    /** Size variant */
    size?: Size;
    /** Status indicator */
    status?: 'online' | 'offline' | 'busy' | 'away';
    /** Border ring */
    ring?: boolean;
    /** Ring color */
    ringColor?: ColorToken;
    /** Additional CSS classes */
    class?: string;
  }

  let {
    src,
    alt,
    name,
    size = 'md',
    status,
    ring = false,
    ringColor = 'teal',
    class: className = '',
  }: Props = $props();

  // State
  let imageError = $state(false);

  // Size configurations
  const sizeConfigs: Record<Size, { avatar: string; text: string; status: string }> = {
    xs: { avatar: 'w-6 h-6', text: 'text-[10px]', status: 'w-1.5 h-1.5' },
    sm: { avatar: 'w-8 h-8', text: 'text-xs', status: 'w-2 h-2' },
    md: { avatar: 'w-10 h-10', text: 'text-sm', status: 'w-2.5 h-2.5' },
    lg: { avatar: 'w-12 h-12', text: 'text-base', status: 'w-3 h-3' },
    xl: { avatar: 'w-16 h-16', text: 'text-lg', status: 'w-3.5 h-3.5' },
    '2xl': { avatar: 'w-20 h-20', text: 'text-xl', status: 'w-4 h-4' },
  };

  // Status color configurations
  const statusConfigs: Record<string, string> = {
    online: 'bg-[hsl(var(--mint-400))]',
    offline: 'bg-[hsl(var(--slate-500))]',
    busy: 'bg-[hsl(var(--coral-400))]',
    away: 'bg-[hsl(var(--amber-400))]',
  };

  // Ring color configurations
  const ringConfigs: Record<string, string> = {
    teal: 'ring-[hsl(var(--teal-500))]',
    coral: 'ring-[hsl(var(--coral-500))]',
    purple: 'ring-[hsl(var(--purple-500))]',
    pink: 'ring-[hsl(var(--pink-500))]',
    mint: 'ring-[hsl(var(--mint-500))]',
    amber: 'ring-[hsl(var(--amber-500))]',
    slate: 'ring-[hsl(var(--slate-500))]',
  };

  // Generate initials from name
  function getInitials(name: string): string {
    const words = name.trim().split(/\s+/);
    if (words.length >= 2) {
      return (words[0][0] + words[words.length - 1][0]).toUpperCase();
    }
    return name.slice(0, 2).toUpperCase();
  }

  // Generate consistent color from name
  function getNameColor(name: string): string {
    const colors = ['teal', 'coral', 'purple', 'pink', 'mint', 'amber'];
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = name.charCodeAt(i) + ((hash << 5) - hash);
    }
    const index = Math.abs(hash) % colors.length;
    return colors[index];
  }

  // Derived values
  let sizeConfig = $derived(sizeConfigs[size]);
  let initials = $derived(name ? getInitials(name) : '?');
  let nameColor = $derived(name ? getNameColor(name) : 'slate');

  // Background color for initials
  const bgColors: Record<string, string> = {
    teal: 'bg-[hsl(var(--teal-600))]',
    coral: 'bg-[hsl(var(--coral-600))]',
    purple: 'bg-[hsl(var(--purple-600))]',
    pink: 'bg-[hsl(var(--pink-600))]',
    mint: 'bg-[hsl(var(--mint-600))]',
    amber: 'bg-[hsl(var(--amber-600))]',
    slate: 'bg-[hsl(var(--slate-600))]',
  };

  let bgColor = $derived(bgColors[nameColor] || bgColors.slate);

  // Ring class
  let baseRingColor = $derived(ringColor.split('-')[0] as string);
  let ringClass = $derived(ring ? `ring-2 ${ringConfigs[baseRingColor] || ringConfigs.teal}` : '');

  // Show image or fallback
  let showImage = $derived(src && !imageError);
</script>

<div class="relative inline-flex {className}">
  <div
    class="relative flex items-center justify-center rounded-full overflow-hidden
           {sizeConfig.avatar}
           {ringClass}"
  >
    {#if showImage}
      <img
        {src}
        alt={alt || name || 'Avatar'}
        class="w-full h-full object-cover"
        onerror={() => imageError = true}
      />
    {:else}
      <div
        class="w-full h-full flex items-center justify-center
               text-white font-medium
               {sizeConfig.text}
               {bgColor}"
      >
        {initials}
      </div>
    {/if}
  </div>

  {#if status}
    <span
      class="absolute bottom-0 right-0 rounded-full border-2 border-[hsl(var(--slate-900))]
             {sizeConfig.status}
             {statusConfigs[status]}"
      aria-label={status}
    ></span>
  {/if}
</div>
