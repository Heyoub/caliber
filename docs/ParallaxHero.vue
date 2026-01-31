<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import NeuralAnimation from './NeuralAnimation.vue';
// @ts-ignore - Import the Rellax library that we installed
import Rellax from 'rellax';
import '../../styles/global.css';
import IcosahedronAnimation from './IcosahedronAnimation.vue';


const props = withDefaults(defineProps<{
  showNeural?: boolean;
  backgroundLogoSrc?: string;
  parallaxLogoSrc?: string;
}>(), {
  showNeural: false,
  backgroundLogoSrc: '/IMG/AXISRA-svg.svg',
  parallaxLogoSrc: '/IMG/axisra-parallax.svg'
});

// Reactive scroll position to share with components
const scrollPosition = ref(0);
const hexagonFactor = ref(1); // 1 = full hexagon, 0 = no hexagon (nodes fill center)

// DOM references for direct manipulation
const sectionRef = ref<HTMLElement | null>(null); // Reference to the entire section
const contentRef = ref<HTMLElement | null>(null); // Reference to the content container
const bgElement = ref<HTMLElement | null>(null);
const fgElement = ref<HTMLElement | null>(null);
const rellaxSection = ref<Rellax | null>(null); // Single Rellax instance for the entire section

// Lifecycle hooks
onMounted(() => {
  // Manually implement parallax scrolling with smooth animations
  const scrollContainer = document.getElementById('parallax-scroll-container');
  
  if (scrollContainer) {
    // Make animations more smooth with request animation frame and better timing
    let ticking = false;
    let lastScrollY = 0;
    let rafId: number | null = null;
    
    // Target values for smoother animation
    const targetValues = {
      bgY: 0,
      fgY: 0,
      contentY: 0
    };
    
    // Current position values with easing
    const currentValues = {
      bgY: 0,
      fgY: 0,
      contentY: 0
    };
    
    // More cohesive speeds - all elements move very similarly but with subtle differences
    const MASTER_SPEED = 0.5;       // Overall speed multiplier
    const CONTENT_SPEED = MASTER_SPEED * 0.85;  // Almost the same as master
    const BG_SPEED = MASTER_SPEED * 0.5;       // Slightly faster
    const FG_SPEED = MASTER_SPEED * 0.75;       // Slightly slower
    
    // Lerp function for smoother transitions
    const lerp = (start: number, end: number, factor: number): number => start + (end - start) * factor;
    
    // Update target positions based on scroll
    const updateTargets = () => {
      const scrollY = scrollContainer.scrollTop || 0;
      
      // Update scroll position for neural animation
      scrollPosition.value = scrollY;
      
      // Calculate hexagon factor - start shrinking after 200px of scroll, disappear by 600px
      const MIN_SCROLL = 150; // Start reducing hexagon after this scroll amount
      const MAX_SCROLL = 650; // Hexagon fully gone (nodes fill center) at this scroll amount
      
      if (scrollY < MIN_SCROLL) {
        hexagonFactor.value = 1; // Full hexagon, no nodes in center
      } else if (scrollY > MAX_SCROLL) {
        hexagonFactor.value = 0; // No hexagon, nodes fill entire center
      } else {
        // Smooth transition between full and gone
        hexagonFactor.value = 1 - ((scrollY - MIN_SCROLL) / (MAX_SCROLL - MIN_SCROLL));
      }
      
      // Update target positions
      targetValues.bgY = scrollY * BG_SPEED;
      targetValues.fgY = scrollY * FG_SPEED;
      targetValues.contentY = scrollY * CONTENT_SPEED;
      
      // Start animation if not already running
      if (!rafId) {
        rafId = requestAnimationFrame(animate);
      }
    };
    
    // Smooth animation loop
    const animate = () => {
      // Ease current values toward targets
      const easing = 0.15; // Lower = smoother but slower
      
      currentValues.bgY = lerp(currentValues.bgY, targetValues.bgY, easing);
      currentValues.fgY = lerp(currentValues.fgY, targetValues.fgY, easing);
      currentValues.contentY = lerp(currentValues.contentY, targetValues.contentY, easing);
      
      // Apply transforms with hardware acceleration
      if (bgElement.value) {
        const element = bgElement.value as HTMLElement;
        element.style.transform = `translate3d(0, ${currentValues.bgY}px, 0)`;
      }
      
      if (fgElement.value) {
        const element = fgElement.value as HTMLElement;
        element.style.transform = `translate3d(0, ${currentValues.fgY}px, 0)`;
      }
      
      if (contentRef.value) {
        const element = contentRef.value as HTMLElement;
        element.style.transform = `translate3d(0, ${currentValues.contentY}px, 0)`;
      }
      
      // Continue animation if values haven't settled
      const isSettled = 
        Math.abs(targetValues.bgY - currentValues.bgY) < 0.1 &&
        Math.abs(targetValues.fgY - currentValues.fgY) < 0.1 &&
        Math.abs(targetValues.contentY - currentValues.contentY) < 0.1;
      
      if (!isSettled) {
        rafId = requestAnimationFrame(animate);
      } else {
        rafId = null;
      }
    };
    
    // Throttled scroll handler using requestAnimationFrame
    const handleScroll = () => {
      if (!ticking) {
        requestAnimationFrame(() => {
          updateTargets();
          ticking = false;
        });
        ticking = true;
      }
    };
    
    // Add event listeners
    scrollContainer.addEventListener('scroll', handleScroll, { passive: true });
    window.addEventListener('resize', updateTargets, { passive: true });
    
    // Force initial update
    updateTargets();
    
    // Setup initial positions and catch any initialization issues
    setTimeout(updateTargets, 100);
    setTimeout(updateTargets, 500);
    
    // Store cleanup function for component unmount
    onUnmounted(() => {
      scrollContainer.removeEventListener('scroll', handleScroll);
    });
  }
});


// Dynamic hexagon calculation that adapts with scroll
// Using function instead of arrow function to resolve excess arguments warning
function isInsideHexagon(x: number, y: number, centerX: number, centerY: number, size: number): boolean {
  // Apply the hexagonFactor to the size check
  const adjustedSize = size * (hexagonFactor.value || 1);
  
  if (adjustedSize <= 0) return false; // No exclusion zone when fully scrolled
  
  const dx = Math.abs(x - centerX);
  const dy = Math.abs(y - centerY);
  
  return (dx <= adjustedSize * Math.cos(Math.PI / 6)) && 
         (dy <= adjustedSize) && 
         (dy <= (adjustedSize * 2 - dx * Math.tan(Math.PI / 6)));
}
</script>

<template>
  <section
    ref="sectionRef"
    class="relative h-[calc(100vh-5rem)] sm:h-[calc(100vh-6rem)] md:h-[calc(100vh-8rem)] w-full max-w-full flex items-center justify-center !m-0 !p-0 overflow-hidden isolate will-change-transform"
  >
    <!-- Background color block -->
    <div class="absolute inset-0 bg-transparent z-[-2]"></div>
    
    <!-- Neural Animation Layer - FIXED (does not move with scroll) -->
    <div v-if="showNeural" class="absolute inset-0 pointer-events-none h-full sm:p-4 md:p-6 z-[1]">
      <NeuralAnimation 
        :intensity="1.2" 
        class="h-full" 
        :density="1.5"
        :scrollY="scrollPosition"
        :hexagonFactor="hexagonFactor"
        :excludeArea="isInsideHexagon"
      />
    </div>

    <!-- Background Logo - Rellax parallax element -->
    <div
      ref="bgElement"
      v-if="backgroundLogoSrc"
      class="rellax-bg absolute inset-0 flex items-center justify-center !m-0 !p-0 pointer-events-none will-change-transform"
    >
      <div class="relative w-[65vw] h-[65vw] sm:w-[50vw] sm:h-[50vw] md:w-[38vw] md:h-[38vw] lg:w-[38vw] lg:h-[38vw] transform scale-100">
        <object
          :data="backgroundLogoSrc"
          type="image/svg+xml"
          class="absolute inset-0 w-full h-full opacity-[0.33] mix-blend-luminosity"
          style="object-fit: contain;"
          preserveAspectRatio="xMidYMid meet"
        ></object>
      </div>
    </div>

    <!-- Content Container (centered) - MOVES with parallax -->
    <div ref="contentRef" class="w-full mx-auto relative px-4 sm:px-6 md:px-8 flex justify-center items-center z-10 will-change-transform">
      <div class="relative w-full max-w-[90%] md:max-w-[80%] pointer-events-auto flex justify-center items-center">
        <slot></slot>
      </div>
    </div>

    <!-- Parallax Overlay Logo - Rellax parallax element -->
    <div 
      ref="fgElement"
      v-if="parallaxLogoSrc" 
      class="rellax-fg absolute inset-0 flex items-center justify-center overflow-hidden !m-0 !p-0 pointer-events-none will-change-transform"
    >
      <div class="relative w-[40vw] h-[40vw] sm:w-[45vw] sm:h-[45vw] md:w-[50vw] md:h-[50vw] lg:w-[50vw] lg:h-[50vw] !max-w-none !max-h-none !min-w-[40vw] sm:!min-w-[45vw] md:!min-w-[50vw] lg:!min-w-[50vw]">
        <img
          :src="parallaxLogoSrc"
          alt="Parallax Logo"
          class="absolute inset-0 w-full h-full object-contain opacity-[.33] select-none mix-blend-plus-lighter transform scale-[1.95]"
        />
      </div>
    </div>
  </section>
</template>

<style>
/* Enhanced hardware acceleration for smoother animations */
.rellax-bg, .rellax-fg, [ref="contentRef"] {
  will-change: transform;
  transform: translateZ(0);
  -webkit-transform: translateZ(0);
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
  transform-style: preserve-3d;
  -webkit-transform-style: preserve-3d;
  perspective: 1000px;
  -webkit-perspective: 1000px;
  contain: paint matterlayout ;
}

/* Force hardware acceleration on all images */
img {
  transform: translateZ(0);
  -webkit-transform: translateZ(0);
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
}

/* Add pointer events none to parallax elements for better performance */
.rellax-bg, .rellax-fg {
  pointer-events: none;
}
</style>

