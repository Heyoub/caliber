<script setup lang="ts">
import { ref, computed } from 'vue';

interface Props {
  variant?: 'default' | 'outline' | 'forgestack';
  size?: 'sm' | 'md' | 'lg';
  href?: string;
  external?: boolean;
  className?: string;
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'default',
  size: 'md',
  href: undefined,
  external: false,
  className: ''
});

const isHovered = ref(false);

const handleMouseEnter = () => {
  isHovered.value = true;
};

const handleMouseLeave = () => {
  isHovered.value = false;
};

const buttonClass = computed(() => {
  const baseClass = 'relative inline-flex items-center justify-center rounded-lg font-medium transition-all duration-200 overflow-hidden';
  
  const sizeClasses = {
    sm: 'text-xs px-3 py-1.5',
    md: 'text-sm px-4 py-2',
    lg: 'text-base px-6 py-3'
  };
  
  const variantClasses = {
    default: 'bg-white/10 text-white border border-white/20 hover:bg-white/20',
    outline: 'bg-transparent text-white border border-white/20 hover:bg-white/10',
    forgestack: 'text-white border-0'
  };

  return [
    baseClass,
    sizeClasses[props.size],
    variantClasses[props.variant],
    props.className
  ].join(' ');
});
</script>

<template>
  <a
    v-if="href"
    :href="href"
    :target="external ? '_blank' : undefined"
    :rel="external ? 'noopener noreferrer' : undefined"
    :class="buttonClass"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
  >
    <!-- ForgeStack gradient background -->
    <div v-if="variant === 'forgestack'" class="absolute inset-0 bg-gradient-to-r from-forgestack-teal/80 to-forgestack-purple/80 opacity-60"></div>
    
    <!-- Hover effects -->
    <div v-if="variant === 'forgestack'" class="absolute inset-0 bg-gradient-to-r from-forgestack-teal to-forgestack-purple opacity-0 transition-opacity duration-300" :class="{ 'opacity-90': isHovered }"></div>
    
    <!-- Button content with proper z-index -->
    <div class="relative z-10 flex items-center gap-2">
      <slot></slot>
    </div>
  </a>
  <button
    v-else
    :class="buttonClass"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
  >
    <!-- ForgeStack gradient background -->
    <div v-if="variant === 'forgestack'" class="absolute inset-0 bg-gradient-to-r from-forgestack-teal/80 to-forgestack-purple/80 opacity-60"></div>
    
    <!-- Hover effects -->
    <div v-if="variant === 'forgestack'" class="absolute inset-0 bg-gradient-to-r from-forgestack-teal to-forgestack-purple opacity-0 transition-opacity duration-300" :class="{ 'opacity-90': isHovered }"></div>
    
    <!-- Button content with proper z-index -->
    <div class="relative z-10 flex items-center gap-2">
      <slot></slot>
    </div>
  </button>
</template>

<style scoped>
/* Add any specific animations here */
button, a {
  -webkit-tap-highlight-color: transparent;
}

/* Add shine effect for forgestack buttons */
button:hover::after,
a:hover::after {
  content: "";
  position: absolute;
  top: 0;
  left: -100%;
  width: 50%;
  height: 100%;
  background: linear-gradient(
    to right,
    transparent,
    rgba(255, 255, 255, 0.2),
    transparent
  );
  transform: skewX(-25deg);
  animation: shine 1.5s infinite;
}

@keyframes shine {
  from {
    left: -100%;
  }
  to {
    left: 200%;
  }
}
</style>
