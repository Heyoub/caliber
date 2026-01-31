<template>
  <button
    :class="[
          'glow-brand-button',
          'group',
          'relative',
          'inline-flex items-center justify-center',
          sizeClass,
          widthClass,
          'font-medium rounded-lg',
          colorClasses.text,
          colorClasses.bg,
          'transition-all duration-300 cursor-pointer whitespace-nowrap border-b-4 overflow-hidden',
          { 'active': isPressed }
        ]"
    :style="buttonStyle"
    @mousedown="handlePressStart"
    @mouseup="handlePressEnd"
    @mouseleave="handlePressEnd"
    @touchstart="handlePressStart"
    @touchend="handlePressEnd"
    @keydown="handleKeyDown"
    @keyup="handleKeyUp"
    @click="$emit('click', $event)"
  >
    <!-- Outer glow effect -->
    <div class="absolute -inset-px rounded-lg opacity-50 transition-all duration-500 ease-out z-[-1] outer-glow-effect"></div>
    <!-- Dark overlay -->
    <div class="absolute inset-0 bg-surface/20 rounded-lg z-[1]"></div>
    <!-- Bevel top effect -->
    <div :class="['absolute inset-x-0 top-0 h-[0.0625rem]', colorClasses.bg, 'rounded-t-lg opacity-0 group-hover:opacity-45 group-active:opacity-100 transition-opacity duration-200 z-[2]']"></div>
    <div :class="['absolute inset-x-0 top-0 h-[0.0625rem]', colorClasses.bevel, 'rounded-t-lg opacity-80 z-[3]']"></div>
    <!-- Subtle base glow -->
    <div :class="['absolute inset-0', colorClasses.useBrandGradient ? 'bg-surface/10' : `bg-[${colorClasses.glow}]/5`, 'opacity-100 group-hover:opacity-0 transition-opacity duration-300 ease-out z-[4]']"></div>
    <!-- Bottom border highlight -->
    <div class="absolute inset-x-0 bottom-0 h-0 opacity-0 group-hover:opacity-100 group-hover:h-1 group-active:h-0 transition-all duration-300 z-[5]" :style="{ backgroundColor: colorClasses.borderColor }"></div>
    <!-- Blob/lava lamp effect background -->
    <div class="absolute inset-0 brand-blob-bg opacity-0 group-hover:opacity-85 transition-all duration-700 ease-out rounded-lg z-[6] overflow-hidden">
      <div v-if="variant === 'coral'" class="absolute inset-0 animate-blob-move opacity-70 variant-blob-coral"></div>
      <div v-if="variant === 'teal'" class="absolute inset-0 animate-blob-move opacity-70 variant-blob-teal"></div>
      <div v-if="variant === 'mint'" class="absolute inset-0 animate-blob-move opacity-70 variant-blob-mint"></div>
      <div v-if="variant === 'lavender'" class="absolute inset-0 animate-blob-move opacity-70 variant-blob-lavender"></div>
      <div v-if="variant === 'slate'" class="absolute inset-0 animate-blob-move opacity-70 variant-blob-slate"></div>
      <div v-if="variant === 'primary'" class="absolute inset-0 animate-blob-move opacity-70 variant-blob-primary"></div>
    </div>
    <!-- Bottom glassmorphic gradient -->
    <div class="absolute bottom-0 left-0 right-0 h-0 glassmorphic-gradient opacity-0 group-hover:opacity-100 group-hover:h-full group-active:h-0 transition-all duration-300 ease-in-out rounded-lg z-[6]"></div>
    <!-- Top glassmorphic gradient -->
    <div class="absolute top-0 left-0 right-0 h-0 glassmorphic-gradient-reverse opacity-0 group-active:opacity-100 group-active:h-[35%] transition-all duration-150 ease-out rounded-lg z-[7] blur-sm"></div>
    <!-- Button text -->
    <span class="relative z-10 flex items-center gap-2 group-active:translate-y-1 transition-transform duration-300">
      <slot />
    </span>
  </button>
</template>

<script setup>
import { ref, computed } from 'vue';

const PRESS_ANIMATION_DURATION = 888;
let pressTimeout = null;
let pressStart = 0;
let isEnding = false;

function usePersistentPress(isPressed) {
  function start() {
    if (!isPressed.value) {
      if (pressTimeout) clearTimeout(pressTimeout);
      isEnding = false;
      isPressed.value = true;
      pressStart = Date.now();
    }
  }
  function end() {
    if (!isPressed.value || isEnding) return;
    isEnding = true;
    const elapsed = Date.now() - pressStart;
    const remaining = PRESS_ANIMATION_DURATION - elapsed;
    if (remaining > 0) {
      pressTimeout = setTimeout(() => {
        isPressed.value = false;
        isEnding = false;
      }, remaining);
    } else {
      isPressed.value = false;
      isEnding = false;
    }
  }
  return { start, end };
}

const props = defineProps({
  variant: { type: String, default: 'coral' },
  size: { type: String, default: 'md' },
  fullWidth: { type: Boolean, default: false },
});
const emit = defineEmits(['click']);
const isPressed = ref(false);
const { start: handlePressStart, end: handlePressEnd } = usePersistentPress(isPressed);
function handleKeyDown(e) {
  if ((e.key === ' ' || e.key === 'Enter') && !isPressed.value) handlePressStart();
}
function handleKeyUp(e) {
  if (e.key === ' ' || e.key === 'Enter') handlePressEnd();
}

const sizeMap = {
  sm: 'px-4 py-2 text-sm',
  md: 'px-6 py-3 text-base',
  lg: 'px-8 py-4 text-lg'
};
const variantMap = {
  teal: {
    bg: 'bg-teal-400',
    text: 'text-white',
    border: 'border-teal-700',
    bevel: 'bg-teal-300',
    glow: 'hsl(190, 90%, 60%)',
    useBrandGradient: false,
    borderColor: '#0f766e',
  },
  coral: {
    bg: 'bg-coral-400',
    text: 'text-white',
    border: 'border-coral-700',
    bevel: 'bg-coral-700',
    glow: 'hsl(0, 90%, 75%)',
    useBrandGradient: false,
    borderColor: '#9f1239',
  },
  mint: {
    bg: 'bg-mint-400',
    text: 'text-white',
    border: 'border-mint-700',
    bevel: 'bg-mint-300',
    glow: 'hsl(165, 65%, 65%)',
    useBrandGradient: false,
    borderColor: '#0d9488',
  },
  lavender: {
    bg: 'bg-lavender-400',
    text: 'text-white',
    border: 'border-lavender-700',
    bevel: 'bg-lavender-300',
    glow: 'hsl(280, 50%, 70%)',
    useBrandGradient: false,
    borderColor: '#6d28d9',
  },
  slate: {
    bg: 'bg-slate-800',
    text: 'text-white',
    border: 'border-gray-700',
    bevel: 'brand-bevel-gradient',
    glow: '#64748b',
    useBrandGradient: true,
    borderColor: '#334155',
  },
  primary: {
    bg: '#4FD1C5',
    text: 'text-white',
    border: 'border-primary/20',
    bevel: 'brand-bevel-gradient',
    glow: '#4FD1C5',
    useBrandGradient: true,
    borderColor: '#4fd1c5',
  }
};
const sizeClass = computed(() => sizeMap[props.size] || sizeMap.md);
const widthClass = computed(() => props.fullWidth ? 'w-full' : '');
const colorClasses = computed(() => variantMap[props.variant] || variantMap.coral);
const buttonStyle = computed(() => ({
  '--glow-color': colorClasses.value.glow,
  '--button-color': `var(--${props.variant})`,
  borderBottomColor: colorClasses.value.borderColor,
}));


</script>

<style scoped>
@keyframes gradient-flow {
  0%, 100% { background-position: 0% 50%; }
  50% { background-position: 100% 50%; }
}
@keyframes blob-move {
  0%, 100% {
    border-radius: 60% 40% 30% 70% / 60% 30% 70% 40%;
    transform: translate(-10px, 10px) scale(1.05);
  }
  25% {
    border-radius: 40% 60% 70% 30% / 50% 60% 30% 60%;
    transform: translate(10px, 10px) scale(1.1);
  }
  50% {
    border-radius: 30% 60% 70% 40% / 50% 60% 30% 60%;
    transform: translate(10px, -10px) scale(1.05);
  }
  75% {
    border-radius: 40% 30% 50% 60% / 30% 40% 60% 50%;
    transform: translate(-10px, -10px) scale(1.1);
  }
}
@keyframes pulse-glow {
  0%, 100% { opacity: 0.4; filter: blur(12px); }
  50% { opacity: 0.7; filter: blur(18px); }
}
.brand-bevel-gradient {
  background: linear-gradient(90deg, var(--brand-1), var(--brand-2), var(--brand-3), var(--brand-4), var(--brand-5), var(--brand-6), var(--brand-7), var(--brand-8));
  animation: gradient-flow 15s ease infinite;
  background-size: 800% 100%;
}
.brand-blob {
  background: linear-gradient(45deg,
    hsl(0, 90%, 75%) 0%,
    hsl(165, 65%, 65%) 25%,
    hsl(190, 92%, 60%) 50%,
    hsl(280, 45%, 65%) 75%,
    hsl(0, 90%, 75%) 100%
  );
  animation: blob-move 20s ease-in-out infinite, gradient-flow 15s ease infinite;
  background-size: 400% 400%;
  filter: blur(15px);
  opacity: 0.6;
  transform-origin: center;
}
.brand-blob-bg {
  background: rgba(15, 23, 42, 0.15);
  backdrop-filter: blur(8px);
}
.outer-glow-effect {
  background: radial-gradient(circle at center, var(--glow-color) 0%, transparent 70%);
  animation: pulse-glow 4s ease-in-out infinite;
  opacity: 0.5;
}
.glassmorphic-gradient {
  background: linear-gradient(to top, rgba(15, 23, 42, 0.4) 0%, rgba(15, 23, 42, 0.2) 40%, rgba(15, 23, 42, 0.05) 100%);
  backdrop-filter: blur(3px);
}
.glassmorphic-gradient-reverse {
  background: linear-gradient(to bottom, rgba(15, 23, 42, 0.5) 0%, rgba(15, 23, 42, 0.25) 40%, rgba(15, 23, 42, 0.1) 100%);
  backdrop-filter: blur(3px);
}
.glow-brand-button:active {
  transform: translateY(4px);
  border-bottom-width: 0px !important;
}
.glow-brand-button:hover::before { opacity: 0.4; }
.glow-brand-button:active::before { opacity: 0; }
</style>
