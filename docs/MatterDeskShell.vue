<template>
  <!-- Abstract AI Dashboard -->
  <div class="relative h-[75vh] md:h-[75vh] mb-0 perspective-1000 overflow-hidden">
    <!-- Abstract gradient glow -->
    <div class="absolute -inset-8 bg-transparent blur-2xl opacity-80 rounded-[3rem] z-0"></div>
    <!-- Abstract floating AI orb -->
    <div class="relative w-full h-full flex flex-col items-center justify-center z-10">
      <div class="relative flex flex-col items-center justify-center">
        <div class="animate-float">
          <div class="w-48 h-48 md:w-64 md:h-64 rounded-full bg-gradient-to-tr from-[#a855f7] via-[#06b6d4] to-[#f59e42] shadow-2xl flex items-center justify-center relative overflow-hidden">
            <!-- AI "core" animation: pulsing orb + subtle waves -->
            <div class="absolute inset-0 flex items-center justify-center">
              <div class="w-24 h-24 md:w-36 md:h-36 rounded-full bg-white/20 backdrop-blur-xl animate-pulse"></div>
            </div>
            <svg class="absolute inset-0 w-full h-full animate-spin-slow opacity-40" viewBox="0 0 200 200" fill="none">
              <ellipse cx="100" cy="100" rx="90" ry="35" stroke="#fff" stroke-width="2" />
              <ellipse cx="100" cy="100" rx="35" ry="90" stroke="#fff" stroke-width="2" />
            </svg>
          </div>
        </div>
        <div class="mt-8 text-center">
          <h1 class="text-3xl md:text-4xl font-bold text-[#a855f7] dark:text-[#c084fc] drop-shadow-lg">ForgeStack Copilot</h1>
          <p class="mt-2 text-base md:text-lg text-[#06b6d4] dark:text-[#22d3ee] font-medium opacity-80">Your abstract, intelligent assistant</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
</script>

<style scoped>
/***** Float animation with subtle rotate *****/
@keyframes floatCard {
  0% {
    transform: translateY(0px) rotate(0deg);
  }
  50% {
    transform: translateY(-12px) rotate(1deg);
  }
  100% {
    transform: translateY(0px) rotate(0deg);
  }
}

.animate-float {
  animation: floatCard 6s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.8;
    transform: scale(1.05);
  }
}

.animate-pulse {
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}
</style>