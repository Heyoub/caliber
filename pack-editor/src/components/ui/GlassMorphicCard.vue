<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  className?: string;
  glowColor?: 'purple' | 'blue' | 'pink' | 'teal' | 'coral' | 'mint' | 'lavender' | 'none';
  hoverEffect?: boolean;
  theme?: 'light' | 'dark';
}

const props = withDefaults(defineProps<Props>(), {
  className: '',
  glowColor: 'none',
  hoverEffect: false,
  theme: 'dark',
});

const glowStyles: Record<string, string> = {
  purple: 'before:bg-[hsla(var(--lavender-500),0.3)]',
  blue: 'before:bg-[hsla(var(--teal-500),0.3)]',
  pink: 'before:bg-pink-500/30',
  teal: 'before:bg-[hsla(var(--teal-500),0.3)]',
  coral: 'before:bg-[hsla(var(--coral-500),0.3)]',
  mint: 'before:bg-[hsla(var(--mint-500),0.3)]',
  lavender: 'before:bg-[hsla(var(--lavender-500),0.3)]',
  none: '',
};

const combinedClass = computed(() => {
  const baseCardClass = props.theme === 'light'
    ? 'bg-white/20 backdrop-blur-md border border-white/30 shadow-lg rounded-2xl'
    : 'bg-slate-900/40 backdrop-blur-md border border-[hsl(var(--teal-500))]/20 shadow-lg rounded-2xl';

  const glowClass = props.glowColor !== 'none'
    ? `before:content-[''] before:absolute before:w-full before:h-full before:top-0 before:left-0 before:opacity-50 before:blur-3xl before:rounded-full before:-z-10 ${glowStyles[props.glowColor]}`
    : '';

  return [
    'relative p-6 overflow-hidden',
    baseCardClass,
    props.hoverEffect && 'hover:scale-[1.02] hover:-translate-y-1 transition-transform duration-300',
    glowClass,
    props.className,
  ].filter(Boolean).join(' ');
});
</script>

<template>
  <div :class="combinedClass">
    <slot />
  </div>
</template>
