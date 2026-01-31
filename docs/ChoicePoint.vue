<script setup lang="ts">
import GlassMorphicCard from '../ui/GlassMorphicCard.vue';
import GlowPressButton from '../ui/GlowPressButton.vue';
import FeaturesCloud from './FeaturesCloud.vue';
import FeaturesPrem from './FeaturesPrem.vue';
import Badge from '@/components/ui/Badge.vue';
import { ref } from 'vue';
import {
  Brain,
  Bolt,
  ArrowRight,
  Split,
  KeySquare,
  ChevronRight
} from 'lucide-vue-next';

// State for path selection (optional, depending on desired functionality)
const selectedPath = ref<string | null>(null);

// State for which features card is shown
const selectedFeatures = ref<string | null>(null);

// Function to handle path selection (can trigger navigation or state change)
function selectPath(path: string) {
  selectedPath.value = path;
  // Materialize the appropriate features card
  if (path === 'focus') {
    selectedFeatures.value = 'cloud';
  } else if (path === 'ownership') {
    selectedFeatures.value = 'prem';
  }
  // TODO: Implement actual navigation or state update based on path
}
</script>

<template>
  <section id="choice-point" class="relative py-24 pb-24 overflow-hidden bg-transparent">
    <!-- Background gradient (now covered by section's own background) -->
    <div class="absolute inset-0 bg-transparent z-[-2
    ]"></div>
    <!-- Animated path connector (visual only) -->
    <div class="bg-transparent absolute left-1/2 top-0 bottom-0 w-1 transform -translate-x-1/2 z-0">
      <div
        class="w-full h-[100vh] bg-transparent"
      ></div>
    </div>
    <!-- Content -->
    <div class="container bg-transparent relative z-10 mx-auto px-4">
      <div
        class="bg-transparent max-w-3xl mx-auto mb-20 text-center"
        
      >
        <Badge 
          appearance="frosted-glass" 
          colorScheme="neutral-slate" 
          :icon="Split" 
          text="Your Cognitive Journey" 
          className="mb-6"
        />
        
        <h2 class="text-4xl md:text-5xl font-bold mb-8 text-navy-800">
          What <span class="text-gradient text-gradient-from-teal-500-to-lavender-500">Matters Most</span> to You?
        </h2>
        
        <p class="text-xl text-navy-700 max-w-2xl mx-auto">
          Everyone's cognitive journey is different. Choose the path that resonates with your biggest challenge.
        </p>
      </div>
      
      <div
        class="bg-transparent flex flex-col md:flex-row gap-16 max-w-6xl mx-auto"
        
      >
        <!-- Focus Path -->
        <GlassMorphicCard
          glowColor="mint"
          hoverEffect
          theme="dark"
          className="md:w-1/2 overflow-hidden cursor-pointer relative"
          :class="selectedPath === 'focus' ? 'ring-4 ring-mint-400 ring-opacity-60' : ''"
        >
          <div class="absolute inset-0 z-20 cursor-pointer" style="background:transparent" @click="selectPath('focus')"></div>
          <!-- Card glow effect -->
          <div class="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 bg-teal-200 blur-xl rounded-xl -m-10"></div>
          <!-- Card content -->
            <!-- Path indicator -->
            <div class="absolute top-0 left-0 w-full h-1 bg-transparent"></div>
            <div class="p-8 text-center h-full flex flex-col">
              <div class="relative inline-block mx-auto">
                <div class="absolute inset-0 bg-teal-100 rounded-full opacity-30 animate-pulse"></div>
                <div class="relative z-10 inline-flex items-center justify-center flex w-20 h-20 bg-gradient-to-br from-teal-500 to-teal-700 rounded-full mb-6 shadow-lg group-hover:scale-110 transition-transform duration-500">
                  <Brain class="h-10 w-10 text-white" :size="40" />
                </div>
              </div>
              <h3 class="text-2xl md:text-3xl font-bold mb-4 text-navy-800">Regaining Mental Focus</h3>
              <p class="text-navy-700 text-lg mb-6 flex-grow">
                I want to eliminate context-switching and create a seamless workflow that matches how I naturally think.
              </p>
              <!-- Feature highlights -->
              <div class="mb-8 space-y-2 text-left">
                <div class="flex items-start space-x-2">
                  <Bolt class="h-5 w-5 text-teal-500 mt-0.5 flex-shrink-0" :size="20" />
                  <span class="text-sm text-navy-700">Reduce context switching by 42%</span>
                </div>
                <div class="flex items-start space-x-2">
                  <Bolt class="h-5 w-5 text-teal-500 mt-0.5 flex-shrink-0" :size="20" />
                  <span class="text-sm text-navy-700">Circular business logic that flows naturally</span>
                </div>
              </div>
              <GlowPressButton
                color="slate"
                :onClick="() => selectPath('focus')"
                size="md"
              >
                <span>Explore This Path</span>
                <ArrowRight class="h-4 w-4" :size="16" />
              </GlowPressButton>
          </div>
        </GlassMorphicCard>
        
        <!-- Ownership Path -->
        <GlassMorphicCard
          glowColor="coral"
          hoverEffect
          theme="dark"
          className="md:w-1/2 overflow-hidden cursor-pointer relative"
          :class="selectedPath === 'ownership' ? 'ring-4 ring-coral-400 ring-opacity-60' : ''"
        >
          <div class="absolute inset-0 z-20 cursor-pointer" style="background:transparent" @click="selectPath('ownership')"></div>
          <!-- Card glow effect -->
          <div class="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 bg-coral-200 blur-xl rounded-xl -m-10"></div>
          <!-- Card content -->
            <!-- Path indicator -->
            <div class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-coral-400 to-coral-600"></div>
            <div class="p-8 text-center h-full flex flex-col">
              <div class="relative inline-block mx-auto">
                <div class="absolute inset-0 bg-coral-100 rounded-full opacity-30 animate-pulse"></div>
                <div class="relative z-10 inline-flex items-center justify-center w-20 h-20 bg-gradient-to-br from-coral-500 to-coral-700 rounded-full mb-6 shadow-lg group-hover:scale-110 transition-transform duration-500">
                  <KeySquare class="h-10 w-10 text-white" :size="40" />
                </div>
              </div>
              <h3 class="text-2xl md:text-3xl font-bold mb-4 text-navy-800">Owning My Tools</h3>
              <p class="text-navy-700 text-lg mb-6 flex-grow">
                I want to break free from endless subscriptions and own my technology infrastructure with more privacy.
              </p>
              <!-- Feature highlights -->
              <div class="mb-8 space-y-2 text-left">
                <div class="flex items-start space-x-2">
                  <Bolt class="h-5 w-5 text-coral-500 mt-0.5 flex-shrink-0" :size="20" />
                  <span class="text-sm text-navy-700">Own your data and tools</span>
                </div>
                <div class="flex items-start space-x-2">
                  <Bolt class="h-5 w-5 text-coral-500 mt-0.5 flex-shrink-0" :size="20" />
                  <span class="text-sm text-navy-700">No more SaaS lock-in</span>
                </div>
              </div>
              <GlowPressButton
                color="slate"
                :onClick="() => selectPath('ownership')"
                size="md"
              >
                <span>Explore This Path</span>
                <ArrowRight class="h-4 w-4" :size="16" />
              </GlowPressButton>
          </div>
        </GlassMorphicCard>
      </div>
      
      <!-- Floating decoration elements -->
      <div class="absolute top-1/4 left-10 w-24 h-24 bg-mint-500 rounded-full opacity-20 blur-2xl"></div>
      <div class="absolute bottom-1/4 right-10 w-32 h-32 bg-coral-500 rounded-full opacity-20 blur-2xl"></div>

      <!-- Debug state output -->
      <div class="text-xs text-red-500 text-center">
        selectedPath: {{ selectedPath }} | selectedFeatures: {{ selectedFeatures }}
      </div>
      <!-- Materialized Features Card -->
      <transition name="fade-slide" mode="out-in">
        <div v-if="selectedFeatures" key="features-card" class="mt-16">
          <GlassMorphicCard
            glowColor="mint"
            theme="dark"
            className="w-full overflow-hidden"
            style="min-height: 300px;"
          >
            <FeaturesCloud v-if="selectedFeatures === 'cloud'" />
            <FeaturesPrem v-else-if="selectedFeatures === 'prem'" />
          </GlassMorphicCard>
        </div>
        <div v-else class="mt-16 w-full text-center text-slate-500 text-lg font-medium opacity-80">
          Select a path above to see features.
        </div>
      </transition>
    </div>
  </section>
</template>

<style scoped>
.fade-slide-enter-active, .fade-slide-leave-active {
  transition: opacity 0.4s cubic-bezier(.4,0,.2,1), transform 0.4s cubic-bezier(.4,0,.2,1);
}
.fade-slide-enter-from, .fade-slide-leave-to {
  opacity: 0;
  transform: translateY(40px);
}
.fade-slide-enter-to, .fade-slide-leave-from {
  opacity: 1;
  transform: translateY(0);
}
</style>