<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';

interface Props {
  glowColor?: 'purple' | 'blue' | 'pink' | 'teal' | 'axisraTeal' | 'axisraPurple' | 'coral' | 'mint' | 'lavender' | 'mintlavender' | 'coralblue' | 'none';
  hoverEffect?: boolean;
  theme?: 'light' | 'dark';
  className?: string;
}

const props = withDefaults(defineProps<Props>(), {
  glowColor: 'none',
  hoverEffect: false,
  theme: 'dark',
  className: ''
});

const cardRef = ref<HTMLElement | null>(null);
const mouseX = ref(0);
const mouseY = ref(0);
const isHovering = ref(false);

// Define glow styles with enhanced hybrid colors
const darkGlowStyles = {
  coral: "before:bg-gradient-to-r before:from-[hsl(var(--coral-500))]/40 before:to-[hsl(var(--coral-600))]/60",
  blue: "before:bg-gradient-to-r before:from-[hsl(var(--blue-500))]/40 before:to-[hsl(var(--blue-600))]/60",  
  mint: "before:bg-gradient-to-r before:from-[hsl(var(--mint-500))]/40 before:to-[hsl(var(--mint-600))]/60",
  lavender: "before:bg-gradient-to-r before:from-[hsl(var(--lavender-500))]/40 before:to-[hsl(var(--lavender-600))]/60",
  mintlavender: "before:bg-gradient-to-r before:from-[hsl(var(--mint-500))]/40 before:to-[hsl(var(--lavender-500))]/60",
  coralblue: "before:bg-gradient-to-r before:from-[hsl(var(--coral-500))]/40 before:to-[hsl(var(--blue-500))]/60",
  purple: "before:bg-gradient-to-r before:from-[hsl(var(--purple-500))]/40 before:to-[hsl(var(--purple-600))]/60",
  pink: "before:bg-gradient-to-r before:from-[hsl(var(--pink-500))]/40 before:to-[hsl(var(--pink-600))]/60",
  teal: "before:bg-gradient-to-r before:from-[hsl(var(--teal-500))]/40 before:to-[hsl(var(--teal-600))]/60",
  axisraTeal: "before:bg-gradient-to-r before:from-[hsl(var(--teal-500))]/40 before:to-[hsl(var(--teal-600))]/60",
  axisraPurple: "before:bg-gradient-to-r before:from-[hsl(var(--purple-500))]/40 before:to-[hsl(var(--purple-600))]/60",
  forgestackPurple: "before:bg-gradient-to-r before:from-[hsl(var(--purple-500))]/40 before:to-[hsl(var(--purple-600))]/60",
  forgestackTeal: "before:bg-gradient-to-r before:from-[hsl(var(--teal-500))]/40 before:to-[hsl(var(--teal-600))]/60",
  none: "",
};


// Define the base card class
const baseCardClass = computed(() => {
  return "hybrid-glass-card";
});

const handleMouseMove = (event: MouseEvent) => {
  if (!cardRef.value) return;
  
  const rect = cardRef.value.getBoundingClientRect();
  mouseX.value = event.clientX - rect.left;
  mouseY.value = event.clientY - rect.top;
  
  // Calculate percentages for gradient position
  const percentX = mouseX.value / rect.width;
  const percentY = mouseY.value / rect.height;
  
  // Update gradient position
  cardRef.value.style.setProperty('--mouse-x', `${percentX * 100}%`);
  cardRef.value.style.setProperty('--mouse-y', `${percentY * 100}%`);
};

const handleMouseEnter = () => {
  isHovering.value = true;
};

const handleMouseLeave = () => {
  isHovering.value = false;
};

onMounted(() => {
  if (cardRef.value) {
    cardRef.value.style.setProperty('--mouse-x', '50%');
    cardRef.value.style.setProperty('--mouse-y', '50%');
  }
});

const classes = computed(() => {
  return [
    "relative p-6 overflow-hidden",
    baseCardClass.value,
    props.hoverEffect && "hover-scale",
    props.glowColor !== "none" &&
      `before:content-[''] before:absolute before:w-full before:h-full before:top-0 before:left-0 before:opacity-70 before:blur-3xl before:rounded-full before:-z-10 ${darkGlowStyles[props.glowColor]}`,
    isHovering.value && "card-hover",
    props.className
  ].filter(Boolean).join(" ");
});
</script>

<template>
  <div 
    :class="classes"
    ref="cardRef"
    @mousemove="handleMouseMove"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
    :data-glow="glowColor"
  >
    <!-- Light effect overlay that matches glow color -->
    <div 
      class="absolute inset-0 opacity-0 transition-opacity duration-300 pointer-events-none light-overlay"
      :class="{ 'opacity-100': isHovering, 
                'light-overlay-blue': glowColor === 'blue',
                'light-overlay-purple': glowColor === 'purple',
                'light-overlay-pink': glowColor === 'pink',
                'light-overlay-teal': glowColor === 'teal',
                'light-overlay-axisraTeal': glowColor === 'axisraTeal' }"
    ></div>
    
    <!-- Content slot -->
    <slot></slot>
  </div>
</template>

<style scoped>
.hybrid-glass-card {
  background: rgba(15, 23, 42, 0.4);
  -webkit-backdrop-filter: blur(14px);
  backdrop-filter: blur(14px);
  border: 1px solid rgba(52, 211, 238, 0.2);
  border-radius: 16px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  color: white;
}

.hover-scale {
  transition: transform 0.3s ease-out;
}

.hover-scale:hover {
  transform: translateY(-5px) scale(1.01);
}

.card-hover {
  border-color: rgba(52, 211, 238, 0.3);
  box-shadow: 
    0 10px 30px rgba(0, 0, 0, 0.2),
    0 0 20px rgba(52, 211, 238, 0.15);
}

/* Base light overlay with saturation effect */
.light-overlay {
  background: radial-gradient(
    circle at var(--mouse-x) var(--mouse-y),
    transparent 0%,
    transparent 70%
  );
  mix-blend-mode: soft-light;
  filter: saturate(1.5) brightness(1.2);
  transition: all 0.3s ease;
}

/* Color-specific overlays */
.light-overlay-blue {
  background: radial-gradient(
    circle at var(--mouse-x) var(--mouse-y),
    rgba(52, 211, 238, 0.3) 0%,
    transparent 70%
  );
  mix-blend-mode: soft-light;
}

.light-overlay-purple {
  background: radial-gradient(
    circle at var(--mouse-x) var(--mouse-y),
    rgba(128, 90, 213, 0.3) 0%,
    transparent 70%
  );
  mix-blend-mode: soft-light;
}

.light-overlay-pink {
  background: radial-gradient(
    circle at var(--mouse-x) var(--mouse-y),
    rgba(213, 63, 140, 0.3) 0%,
    transparent 70%
  );
  mix-blend-mode: soft-light;
}

.light-overlay-teal {
  background: radial-gradient(
    circle at var(--mouse-x) var(--mouse-y),
    rgba(79, 209, 197, 0.3) 0%,
    transparent 70%
  );
  mix-blend-mode: soft-light;
}

.light-overlay-axisraTeal {
  background: radial-gradient(
    circle at var(--mouse-x) var(--mouse-y),
    rgba(52, 211, 238, 0.3) 0%,
    transparent 70%
  );
  mix-blend-mode: soft-light;
}

.light-overlay-axisraPurple {
  background: radial-gradient(
    circle at var(--mouse-x) var(--mouse-y),
    rgba(128, 90, 213, 0.3) 0%,
    transparent 70%
  );
  mix-blend-mode: soft-light;
}

/* Enhanced border glow for cards */
.card-hover {
  box-shadow: 
    0 10px 30px rgba(0, 0, 0, 0.2),
    0 0 20px rgba(52, 211, 238, 0.2), 
    0 0 0 1px rgba(52, 211, 238, 0.3);
}

/* Enhanced color effects by glow color */
.hybrid-glass-card[data-glow="blue"] {
  box-shadow: 
    0 10px 30px rgba(0, 0, 0, 0.2),
    0 0 20px rgba(52, 211, 238, 0.2), 
    0 0 0 1px rgba(52, 211, 238, 0.3);
}

.hybrid-glass-card[data-glow="purple"] {
  box-shadow: 
    0 10px 30px rgba(0, 0, 0, 0.2),
    0 0 20px rgba(128, 90, 213, 0.2), 
    0 0 0 1px rgba(128, 90, 213, 0.3);
}

.hybrid-glass-card[data-glow="pink"] {
  box-shadow: 
    0 10px 30px rgba(0, 0, 0, 0.2),
    0 0 20px rgba(213, 63, 140, 0.2), 
    0 0 0 1px rgba(213, 63, 140, 0.3);
}

.hybrid-glass-card[data-glow="teal"], 
.hybrid-glass-card[data-glow="axisraTeal"] {
  box-shadow: 
    0 10px 30px rgba(0, 0, 0, 0.2),
    0 0 20px rgba(79, 209, 197, 0.2), 
    0 0 0 1px rgba(79, 209, 197, 0.3);
}
</style>
