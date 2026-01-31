<template>
      <!-- Title Banner Section - Wide -->
      <div class="w-full bg-slate-800 border-t border-white/5 border-solid py-24 rounded-t-3xl">
      <div class="container mx-auto max-w-8xl z-20 relative px-0">
        <div class="relative flex flex-col md:flex-col items-center justify-between gap-0 bg-gradient-to-br from-[#a855f7]/30 via-[#06b6d4]/20 to-[#f59e42]/30 dark:from-[#a855f7]/50 dark:via-[#06b6d4]/30 dark:to-[#f59e42]/50 backdrop-blur-xl p-0 rounded-2xl border border-[#34D3EE]/20 shadow-lg fade-in visible overflow-hidden"
          style="box-shadow: 0 0 25px rgba(52, 211, 238, 0.1), 0 0 15px rgba(52, 211, 238, 0.05);">
          <!-- Animated SVG Background (Moved here) -->
          <div class="absolute justify-center flex items-center inset-0 opacity-30 z-[-1]">
            <img
              src="/IMG/axisra-tech-animation.svg"
              alt=""
              class="w-full h-full object-cover scale-[1.1] sm:scale-100"
              loading="lazy"
              style="transform-origin: center center;"
            />
          </div>
          <Cta />      
        </div>
      </div>
    </div>
</template>

<script setup lang="ts">
import Cta from '@/components/layout/Cta.vue';
</script>