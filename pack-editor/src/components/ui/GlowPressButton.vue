<script setup lang="ts">
import { computed } from 'vue';
import type { PropType } from 'vue';

interface ColorConfig {
  bg: string;
  hover: string;
  text: string;
  border: string;
  hoverBorder: string;
  shadow: string;
  gradient: string;
  pressGradient: string;
  glow: string;
  outerGlow: string;
}

const props = defineProps({
  href: { type: String, default: '#' },
  onClick: { type: Function as PropType<() => void> },
  color: {
    type: String as PropType<'teal'|'coral'|'mint'|'lavender'|'primary'|'slate'|'glassy'|'danger'>,
    default: 'primary'
  },
  size: { type: String as PropType<'sm'|'md'|'lg'>, default: 'md' },
  fullWidth: { type: Boolean, default: false },
  type: { type: String as PropType<'button'|'submit'|'reset'>, default: 'button' },
  forcePressed: { type: Boolean, default: false },
  disabled: { type: Boolean, default: false }
});

// Color mapping with enhanced gradients and glows
const colorMap: Record<string, ColorConfig> = {
  teal: {
    bg: 'bg-[hsl(var(--teal-500))]',
    hover: 'hover:bg-[hsl(var(--teal-600))]',
    text: 'text-white',
    border: 'border-[hsl(var(--teal-700))]',
    hoverBorder: 'hover:border-[hsl(var(--teal-800))]',
    shadow: 'shadow-[0_0_8px_hsl(var(--teal-400)_/_0.4)]',
    gradient: 'from-[hsl(var(--teal-300))] via-[hsl(var(--teal-500))] to-[hsl(var(--teal-700))]',
    pressGradient: 'from-[hsl(var(--teal-700))] via-[hsl(var(--teal-500))] to-[hsl(var(--teal-300))]',
    glow: 'hsl(var(--teal-400))',
    outerGlow: 'rgba(45, 212, 191, 0.4)'
  },
  coral: {
    bg: 'bg-[hsl(var(--coral-500))]',
    hover: 'hover:bg-[hsl(var(--coral-600))]',
    text: 'text-white',
    border: 'border-[hsl(var(--coral-300))]',
    hoverBorder: 'hover:border-[hsl(var(--coral-300))]',
    shadow: 'shadow-[0_0_8px_hsl(var(--coral-400)_/_0.4)]',
    gradient: 'from-[hsl(var(--coral-300))] via-[hsl(var(--coral-500))] to-[hsl(var(--coral-700))]',
    pressGradient: 'from-[hsl(var(--coral-700))] via-[hsl(var(--coral-500))] to-[hsl(var(--coral-300))]',
    glow: 'hsl(var(--coral-400))',
    outerGlow: 'rgba(255, 129, 112, 0.4)'
  },
  mint: {
    bg: 'bg-[hsl(var(--mint-500))]',
    hover: 'hover:bg-[hsl(var(--mint-600))]',
    text: 'text-white',
    border: 'border-[hsl(var(--mint-700))]',
    hoverBorder: 'hover:border-[hsl(var(--mint-800))]',
    shadow: 'shadow-[0_0_8px_hsl(var(--mint-400)_/_0.4)]',
    gradient: 'from-[hsl(var(--mint-300))] via-[hsl(var(--mint-500))] to-[hsl(var(--mint-700))]',
    pressGradient: 'from-[hsl(var(--mint-700))] via-[hsl(var(--mint-500))] to-[hsl(var(--mint-300))]',
    glow: 'hsl(var(--mint-400))',
    outerGlow: 'rgba(110, 231, 183, 0.4)'
  },
  lavender: {
    bg: 'bg-[hsl(var(--lavender-500))]',
    hover: 'hover:bg-[hsl(var(--lavender-600))]',
    text: 'text-white',
    border: 'border-[hsl(var(--lavender-700))]',
    hoverBorder: 'hover:border-[hsl(var(--lavender-800))]',
    shadow: 'shadow-[0_0_8px_hsl(var(--lavender-400)_/_0.4)]',
    gradient: 'from-[hsl(var(--lavender-300))] via-[hsl(var(--lavender-500))] to-[hsl(var(--lavender-700))]',
    pressGradient: 'from-[hsl(var(--lavender-700))] via-[hsl(var(--lavender-500))] to-[hsl(var(--lavender-300))]',
    glow: 'hsl(var(--lavender-400))',
    outerGlow: 'rgba(167, 139, 250, 0.4)'
  },
  primary: {
    bg: 'bg-[#1A2E35]',
    hover: 'hover:bg-[#141f23]',
    text: 'text-[#4FD1C5]',
    border: 'border-[#4FD1C5]/20',
    hoverBorder: 'hover:border-[#4FD1C5]',
    shadow: 'shadow-[0_0_8px_hsl(var(--mint-400)_/_0.6),0_0_16px_hsl(var(--lavender-500)_/_0.6)]',
    gradient: 'from-[hsl(var(--coral-400))] via-[hsl(var(--mint-400))] to-[hsl(var(--lavender-500))]',
    pressGradient: 'from-[hsl(var(--lavender-500))] via-[hsl(var(--mint-400))] to-[hsl(var(--coral-400))]',
    glow: '#4FD1C5',
    outerGlow: 'rgba(79, 209, 197, 0.5)'
  },
  slate: {
    bg: 'bg-slate-800',
    hover: 'hover:bg-slate-900',
    text: 'text-slate-400',
    border: 'border-slate-700',
    hoverBorder: 'hover:border-slate-600',
    shadow: 'shadow-[0_0_8px_rgba(51,65,85,0.4)]',
    gradient: 'from-slate-700 via-slate-800 to-slate-900',
    pressGradient: 'from-slate-900 via-slate-800 to-slate-700',
    glow: '#334155',
    outerGlow: 'rgba(51, 65, 85, 0.4)'
  },
  glassy: {
    bg: 'bg-white/10',
    hover: 'hover:bg-white/20',
    text: 'text-white',
    border: 'border-white/20',
    hoverBorder: 'hover:border-white/30',
    shadow: 'shadow-[0_0.25rem_0.75rem_rgba(0,0,0,0.2)]',
    gradient: 'from-white/10 via-transparent to-transparent',
    pressGradient: 'from-black/20 via-transparent to-transparent',
    glow: 'rgba(255, 255, 255, 0.3)',
    outerGlow: 'rgba(79, 209, 197, 0.3)'
  },
  danger: {
    bg: 'bg-red-600',
    hover: 'hover:bg-red-700',
    text: 'text-white',
    border: 'border-red-500',
    hoverBorder: 'hover:border-red-400',
    shadow: 'shadow-[0_0_8px_rgba(239,68,68,0.4)]',
    gradient: 'from-red-400 via-red-600 to-red-800',
    pressGradient: 'from-red-800 via-red-600 to-red-400',
    glow: '#ef4444',
    outerGlow: 'rgba(239, 68, 68, 0.4)'
  }
};

const selectedColor = computed(() => colorMap[props.color] || colorMap.primary);

const sizeMap = { sm: 'px-4 py-2 text-sm', md: 'px-6 py-3 text-base', lg: 'px-8 py-4 text-lg' };
const selectedSize = computed(() => sizeMap[props.size]);
const widthClass = computed(() => props.fullWidth ? 'w-full' : '');

const isLink = computed(() => props.href && props.href !== '#');
const Tag = computed(() => isLink.value ? 'a' : 'button');
const buttonType = computed(() => !isLink.value ? props.type : undefined);
const linkHref = computed(() => isLink.value ? props.href : undefined);
</script>

<template>
  <component
    :is="Tag"
    :href="linkHref"
    :type="buttonType"
    :disabled="disabled"
    :class="[
      'glow-press-button group relative inline-flex items-center justify-center',
      selectedSize,
      widthClass,
      'font-medium rounded-lg', selectedColor.text,
      selectedColor.bg, 'transition-all duration-300 cursor-pointer whitespace-nowrap border-b-4',
      selectedColor.border, selectedColor.hoverBorder, 'overflow-hidden',
      props.forcePressed ? 'pressed border-b-2' : 'active:border-b-2',
      props.disabled ? 'opacity-50 cursor-not-allowed' : ''
    ]"
    :style="{
      '--glow-color': selectedColor.glow,
      '--outer-glow-color': selectedColor.outerGlow
    }"
    @click="!disabled && onClick?.()"
    :data-pressed="props.forcePressed"
    :aria-pressed="props.forcePressed"
  >
    <!-- Enhanced outer glow -->
    <div class="absolute -inset-px rounded-lg opacity-50 transition-all duration-500 ease-out z-[-1] outer-glow-effect"
         :class="{'forced-glow': props.forcePressed}"></div>

    <!-- Border highlight -->
    <div class="absolute inset-0 border border-[#4FD1C5]/20 group-hover:border-[#4FD1C5]/50 rounded-lg transition-all duration-300 z-[1]"
         :class="{'border-[#4FD1C5]/60': props.forcePressed}"></div>

    <!-- Top edge highlight -->
    <div class="absolute inset-x-0 top-0 h-[0.0625rem] bg-slate-800/70 rounded-t-lg group-hover:bg-slate-700/80 transition-colors duration-300 z-[4]"
         :class="{'bg-slate-700/90': props.forcePressed}"></div>

    <!-- Darkening overlay -->
    <div class="absolute inset-0 bg-black/10 opacity-100 group-hover:opacity-0 transition-opacity duration-300 ease-out z-[3]"
         :class="{'opacity-0': props.forcePressed}"></div>

    <!-- Animated gradient background -->
    <div class="absolute inset-0 bg-gradient-to-br transition-all duration-700 ease-out animate-gradient-flow z-[4]"
         :class="selectedColor.gradient + (props.forcePressed ? ' opacity-60' : ' opacity-0 group-hover:opacity-75')"></div>

    <!-- Pressed gradient -->
    <div class="absolute inset-0 bg-gradient-to-tr transition-all duration-300 ease-out z-[5]"
         :class="selectedColor.pressGradient + (props.forcePressed ? ' opacity-66' : ' opacity-0 group-active:opacity-66')"></div>

    <!-- Bottom glass effect -->
    <div class="absolute bottom-0 left-0 right-0 glassmorphic-gradient transition-all duration-300 ease-in-out rounded-lg z-[6]"
         :class="props.forcePressed ? 'opacity-0 h-0' : 'opacity-0 h-0 group-hover:opacity-100 group-hover:h-full group-active:h-0'"></div>

    <!-- Top glass effect -->
    <div class="absolute top-0 left-0 right-0 glassmorphic-gradient-reverse transition-all duration-200 ease-out rounded-lg z-[7]"
         :class="props.forcePressed ? 'opacity-100 h-[70%]' : 'opacity-0 h-0 group-active:opacity-100 group-active:h-[70%]'"></div>

    <!-- Content -->
    <span class="relative z-10 flex items-center gap-2 transition-transform duration-150"
          :class="props.forcePressed ? 'translate-y-px' : 'group-active:translate-y-px'">
      <slot />
    </span>
  </component>
</template>

<style scoped>
@keyframes gradient-flow { 0%,100%{background-position:0% 50%}50%{background-position:100% 50%} }
@keyframes pulse-glow {0%,100%{opacity:0.2;filter:blur(8px)}50%{opacity:0.4;filter:blur(14px)}}

.animate-gradient-flow{animation:gradient-flow 8s ease infinite;background-size:300% 300%;}

.glow-press-button::before{
  content:'';
  position:absolute;
  inset:-2px;
  background:radial-gradient(circle at center,var(--glow-color),transparent 70%);
  opacity:0.14;
  transition:all 0.5s;
  border-radius:inherit;
  filter:blur(10px);
  z-index:-1;
  animation:pulse-glow 4s ease-in-out infinite;
}

.glow-press-button:hover::before{
  opacity:0.45;
  filter:blur(14px);
}

.glow-press-button:active::before{
  opacity:0.2;
  transform: scale(0.95) translateY(3px);
}

.outer-glow-effect{
  box-shadow:0 0 12px 3px var(--outer-glow-color);
  opacity:0.15;
}

.glow-press-button:hover .outer-glow-effect{
  box-shadow:0 0 18px 5px var(--outer-glow-color), 0 0 30px 10px var(--outer-glow-color);
  opacity:0.3;
}

.glassmorphic-gradient{
  background:linear-gradient(to top,rgba(0,0,0,0.25) 0%,rgba(0,0,0,0.15) 40%,transparent 100%);
  backdrop-filter:blur(3px);
}

.glassmorphic-gradient-reverse{
  background:linear-gradient(to bottom,rgba(0,0,0,0.3) 0%,rgba(0,0,0,0.15) 40%,transparent 100%);
  backdrop-filter:blur(3px);
}

.glow-press-button:is(button):focus-visible {
  outline: 2px solid var(--glow-color);
  outline-offset: 2px;
}

.glow-press-button[data-pressed="true"] {
  transform: translateY(4px);
}

.forced-glow {
  box-shadow: 0 0 15px 4px var(--outer-glow-color), 0 0 25px 8px var(--outer-glow-color);
  opacity: 0.35 !important;
}

.pressed {
  box-shadow: inset 0 8px 10px rgba(0, 0, 0, 0.4);
  transform: translateY(4px);
}
</style>
