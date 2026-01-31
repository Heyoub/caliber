<template class="bg-gradient-to-br from-[#a855f7]/20 via-[#06b6d4]/20 to-[#f59e42]/40 dark:from-[#a855f7]/40 dark:via-[#06b6d4]/30 dark:to-[#f59e42]/40">
  <section class="bg-gradient-to-br from-[#a855f7]/20 via-[#06b6d4]/20 to-[#f59e42]/40 dark:from-[#a855f7]/40 dark:via-[#06b6d4]/30 dark:to-[#f59e42]/40">
  <Hero
    title="ForgeStack"
    subtitle="Our tool think with you; not at you."
    :showNeural="true"
    :cleanSubtitleStyle="true"
  />
  <!--div class="absolute -inset-8 bg-gradient-to-br from-[#a855f7]/30 via-[#06b6d4]/20 to-[#f59e42]/30 dark:from-[#a855f7]/50 dark:via-[#06b6d4]/30 dark:to-[#f59e42]/50 blur-2xl opacity-80 rounded-[3rem] z-0">-->
<section class="w-full bg-slate-700 border-t border-white/5 border-solid py-24 rounded-3xl">
      <div class="container mx-auto max-w-8xl z-20 relative px-0 bg-gradient-to-br from-[#a855f7]/30 via-[#06b6d4]/20 to-[#f59e42]/30 dark:from-[#a855f7]/50 dark:via-[#06b6d4]/30 dark:to-[#f59e42]/50 rounded-3xl">
        <div class="relative flex flex-col md:flex-col items-center justify-between gap-0 bg-transparent backdrop-blur-xl p-0 rounded-2xl border border-[#34D3EE]/20 shadow-lg fade-in visible overflow-hidden"
          style="box-shadow: 0 0 25px rgba(52, 211, 238, 0.1), 0 0 15px rgba(52, 211, 238, 0.05);">
          <!-- Animated SVG Background (Moved here) -->
          <div class="absolute justify-center flex items-center inset-0 opacity-30 z-[-1]">
            <img
              src="/IMG/axisra-tech-animation copy.svg"
              alt=""
              class="w-full h-full object-cover scale-[1.1] sm:scale-100"
              loading="lazy"
              style="transform-origin: center center;"
            />
          </div>
          <!-- Title Side -->
          <Problem />
        </div>  
      </div>
    </section>
    <div class="bg-transparent max-h-[100vh] absolute top-0 left-0 right-0 z-[10]"></div>
    <div class="absolute inset-0 bg-gradient-calm opacity-10 z-[-2]"></div>
    <div class="absolute inset-0 bg-gradient-mesh opacity-10 z-[-1]"></div>
    <!-- Background elements -->
    <div class="absolute inset-0 bg-gradient-to-b from-[hsl(var(--navy-800))] to-[hsl(var(--navy-900))] opacity-10 z-[-2]"></div>
    <div class="absolute top-40 right-10 w-64 h-64 bg-[hsl(var(--teal-800))] rounded-full opacity-10 blur-3xl"></div>
    <div class="absolute bottom-20 left-10 w-72 h-72 bg-[hsl(var(--mint-800))] rounded-full opacity-10 blur-3xl"></div>
    <div class="mb-16"></div>
        <!-- Foreground Element with Parallax -->
        <!-- Parallax Dashboob Section with enhanced parallax -->
        <!-- Stats -->
        <div class="mt-16 text-center">
          <Badge
            appearance="frosted-glass"
            colorScheme="neutral-slate"
            :icon="Sparkles"
            iconSize="sm"
            text="A cognitive sanctuary for small-business work—software that reduces effort instead of demanding more."
            textSize="base"
          />
        </div>
        <div class="will-change-transform">
          <DashboobVue />
        </div>
        <!-- Stats -->
        <div class="mt-16 text-center">
          <Badge
            appearance="frosted-glass"
            colorScheme="neutral-slate"
            :icon="Zap"
            iconSize="sm"
            text="Businesses reduce cognitive switch-cost by 42% and reclaim 3 hours daily"
            textSize="sm"
          />
        </div>
    <Journey />  
      <!-- Title Banner Section - Wide -->
    <div class="w-full bg-slate-700 border-t border-white/5 border-solid py-24 rounded-t-3xl">
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
          <CtaList />      
        </div>
      </div>
    </div>
    </section>
</template>

<script setup lang="ts">
import CtaList from '../components/layout/CtaList.vue';
import Hero from '../components/core/Hero.vue';
import FeaturesCloud from '../components/core/FeaturesCloud.vue';
import FeaturesPrem from '../components/core/FeaturesPrem.vue';
import Problem from '../components/core/Problem.vue';
import Journey from '../components/core/Journey.vue';
import HowItWorks from '../components/core/HowItWorks.vue';
import DashboobVue from '../components/core/Dashboob.vue';
import Badge from '@/components/ui/Badge.vue';
import { Sparkles as SparklesIcon, Zap as ZapIcon } from 'lucide-vue-next'; // Alias imports
import { markRaw } from 'vue'; // Import markRaw

// Mark icons as raw
const Sparkles = markRaw(SparklesIcon);
const Zap = markRaw(ZapIcon);

// Add animation triggers
const observerOptions = {
    root: null,
    rootMargin: "0px",
    threshold: 0.1,
};

const observer = new IntersectionObserver((entries) => {
    entries.forEach((entry) => {
      if (entry.isIntersecting) {
        entry.target.classList.add("visible");
      }
    });
}, observerOptions);

// Observe all elements with fade-in class
document.querySelectorAll('.fade-in').forEach(el => observer.observe(el));

// Wrap words in spans for hover effect
document.querySelectorAll('.hover-text p, .hover-text li').forEach(element => {
    const text = element.textContent;
    if (text && !element.querySelector('a')) {
      element.innerHTML = text.split(' ').map(word =>
        `<span>${word}</span>`
      ).join(' ');
    }
  });
</script>

<style scoped>
/* it-backface-visibility: hidden;
} */
p-2 border-t border-slate-800 border-solid mt-auto
/* Add pointer events none to parallax elements for better performance, if appropriate */
 [ref="bgElement"], [ref="fgElement"] {
  pointer-events: none;
} 

.text-gradient {
  background-clip: text;
  -webkit-background-clip: text;
  color: transparent;
}

.text-gradient-primary {
   background-image: linear-gradient(to right, hsl(var(--teal-500)), hsl(var(--mint-500)), hsl(var(--lavender-500)));
}

@keyframes text-pulse {
  50% { opacity: 0.7; }
}
.animate-text-pulse {
  animation: text-pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-10px); }
}
.animate-float {
  animation: float 6s ease-in-out infinite;
}
</style>
