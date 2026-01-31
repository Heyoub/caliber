<script lang="ts">
  /**
   * CircularRings - Decorative pulsing concentric rings animation
   * Reference: Vue CircularRings.vue
   */
  interface Props {
    /** Size of the container */
    size?: 'sm' | 'md' | 'lg' | 'xl';
    /** Ring colors (4 colors for 4 rings) */
    colors?: [string, string, string, string];
    /** Animation duration in seconds */
    duration?: number;
    /** Additional CSS classes */
    class?: string;
  }

  let {
    size = 'md',
    colors = [
      'rgba(45, 212, 191, 0.25)',  // teal
      'rgba(110, 231, 183, 0.25)', // mint
      'rgba(167, 139, 250, 0.25)', // purple
      'rgba(244, 63, 94, 0.25)',   // coral
    ],
    duration = 2.5,
    class: className = '',
  }: Props = $props();

  const sizeConfig = {
    sm: { container: 'w-16 h-16 p-4', rings: [60, 45, 30, 15] },
    md: { container: 'w-[120px] h-[120px] p-24', rings: [120, 90, 60, 30] },
    lg: { container: 'w-40 h-40 p-32', rings: [160, 120, 80, 40] },
    xl: { container: 'w-56 h-56 p-44', rings: [224, 168, 112, 56] },
  };

  let config = $derived(sizeConfig[size]);
</script>

<div
  class="circular-rings relative flex items-center justify-center {config.container} {className}"
  style="--ring-duration: {duration}s;"
  role="presentation"
  aria-hidden="true"
>
  {#each config.rings as ringSize, index}
    <div
      class="ring absolute rounded-full border-2"
      style="
        width: {ringSize}px;
        height: {ringSize}px;
        border-color: {colors[index]};
        animation-delay: {index * (duration / 4)}s;
      "
    ></div>
  {/each}
</div>

<style>
  .ring {
    animation: pulse-ring var(--ring-duration) cubic-bezier(0.4, 0, 0.2, 1) infinite;
  }

  @keyframes pulse-ring {
    0%, 100% {
      opacity: 0.8;
      transform: scale(1);
    }
    50% {
      opacity: 0.2;
      transform: scale(1.08);
    }
  }
</style>
