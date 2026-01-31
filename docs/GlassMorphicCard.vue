<template>
  <div :class="combinedClass" :style="style">
    <slot />
  </div>
</template>

<script setup lang="ts">
import { computed, defineProps, withDefaults } from 'vue';

// Define the props interface
interface Props {
  className?: string;
  glowColor?: 'purple' | 'blue' | 'pink' | 'teal' | 'forgestackPurple' | 'forgestackTeal' | 'none' | 'coral' | 'mint' | 'lavender';
  hoverEffect?: boolean;
  style?: Record<string, any>;
  theme?: 'light' | 'dark';
}

// Define props with defaults
const props = withDefaults(defineProps<Props>(), {
  className: '',
  glowColor: 'none',
  hoverEffect: false,
  style: () => ({}),
  theme: 'light', // Default to light theme
});

// Glow styles based on theme
const lightGlowStyles = {
  purple: 'before:bg-matterPurple/20',
  blue: 'before:bg-matterBlue/20',
  pink: 'before:bg-matterPink/20',
  teal: 'before:bg-forgestack-teal/20',
  forgestackPurple: 'before:bg-forgestack-purple/20',
  forgestackTeal: 'before:bg-forgestack-teal/20',
  coral: 'before:bg-[hsla(var(--coral-500),0.2)]',
  mint: 'before:bg-[hsla(var(--mint-500),0.2)]',
  lavender: 'before:bg-[hsla(var(--lavender-500),0.2)]',
  none: '',
};

const darkGlowStyles = {
  purple: 'before:bg-matterPurple/30',
  blue: 'before:bg-matterBlue/30',
  pink: 'before:bg-matterPink/30',
  teal: 'before:bg-forgestack-teal/30',
  forgestackPurple: 'before:bg-forgestack-purple/30',
  forgestackTeal: 'before:bg-forgestack-teal/30',
  coral: 'before:bg-[hsla(var(--coral-500),0.3)]',
  mint: 'before:bg-[hsla(var(--mint-500),0.3)]',
  lavender: 'before:bg-[hsla(var(--lavender-500),0.3)]',
  none: '',
};

// Computed property for the final class string
const combinedClass = computed(() => {
  const glowStyles = props.theme === 'light' ? lightGlowStyles : darkGlowStyles;
  const baseCardClass = props.theme === 'light'
    ? 'glass-card' // Assumes 'glass-card' class is defined globally or imported for light theme
    : 'bg-[#1A2E35]/40 backdrop-blur-md border border-forgestack-teal/20 shadow-lg rounded-2xl'; // Dark theme styles

  const glowClass = props.glowColor !== 'none'
    ? `before:content-[''] before:absolute before:w-full before:h-full before:top-0 before:left-0 before:opacity-50 before:blur-3xl before:rounded-full before:-z-10 ${glowStyles[props.glowColor]}`
    : '';

  return [
    'relative p-6 overflow-hidden',
    baseCardClass,
    props.hoverEffect && 'hover-scale', // Assumes 'hover-scale' class is defined globally or imported
    glowClass,
    props.className,
  ].filter(Boolean).join(' ');
});

</script>

<style scoped>
/* Scoped styles specific to GlassMorphicCard if any */
/* Define .glass-card and .hover-scale globally if they are not already */

/* Example placeholder for global styles - these should ideally be in a global CSS file */
/*
.glass-card {
  background: rgba(255, 255, 255, 0.2);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 1rem; 
}

.hover-scale {
  transition: transform 0.3s ease;
}
.hover-scale:hover {
  transform: scale(1.03);
}
*/
</style>
