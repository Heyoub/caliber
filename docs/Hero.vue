<template>
  <ParallaxHero :show-neural="props.showNeural">
    <div :class="['w-full mx-auto pt-8 md:pt-1', props.className]">
      <!-- Container for Centering Content -->
      <div class="w-full text-center">
        <div class="max-w-[90vw] sm:max-w-[80vw] md:max-w-[70vmin] mx-auto px-4 sm:px-6 md:px-8">

          <!-- Logo Animation with Glow Box -->
          <div class="mx-auto mt-16 mb-4 animate-pulse-slow">
            <LogoAnimation />
          </div>
          <div class="text-center mb-12">
          <div class="inline-block mb-6 p-3 bg-gradient-to-b-to-b from-mint-500/30 to-coral-500/25 backdrop-blur-xl rounded-2xl">
            <span class="text-sm font-semibold px-4 py-2 bg-gradient-to-t from-mint-500/10 to-lavender-500/25 text-white rounded-xl">
              Cognitive-First Business Platform
            </span>
          </div>
          <h1 class="text-5xl md:text-6xl lg:text-7xl font-bold leading-tight mb-6">
            <span class="text-white">Software That Feels Like </span><span class="relative text-gradient text-gradient-primary animate-text-pulse">Thinking</span>
          </h1>
        </div>
          <!-- Text Content -->
          <!--<StyledHeading 
            :level="1" 
            size="hero" 
            :class-name="`mx-0 mb-3 text-4xl md:text-6xl ${props.titleClassName}`"
          >
            {{ props.title }}
          </StyledHeading>

          <p v-if="props.byline" class="text-sm md:text-base text-gray-400 italic mb-3">
            <BrandGradient as="span">{{ props.byline }}</BrandGradient>
          </p>-->

          <!-- Subtitle logic -->
        <p v-if="props.cleanSubtitleStyle" 
             class="text-white/80 mb-3 px-8 text-lg text-center mx-auto max-w-2xl" 
             v-html="props.subtitle">
          </p>
          <BrandGradient 
            v-else
            as="p" 
            class="text-base md:text-lg leading-relaxed [text-wrap:pretty] animate-gradient-flow opacity-75 mb-8"
            style="text-shadow: 0 0 8px rgba(79,209,197,0.9), 0 0 16px rgba(128,90,213,0.9), 0 0 20px rgba(128,90,213,0.9);"
            v-html="props.subtitle">
          </BrandGradient> 
          
          <div class="pt-0 -mt-4">
            <img
              src="/IMG/arrow down.gif"
              alt="Scroll down"
              class="w-24 h-48 mx-auto animate-float filter-arrow"
              />
          </div>
        </div>
      </div>
    </div>
  </ParallaxHero>
</template>

<script setup lang="ts">
import ParallaxHero from './ParallaxHero.vue';
import StyledHeading from '../ui/StyledHeading.vue'; 
import BrandGradient from '../ui/BrandGradient.vue'; 
import LogoAnimation from './LogoAnimation.vue'; 

interface Props {
  title: string;
  subtitle: string; 
  byline?: string;
  className?: string;
  titleClassName?: string;
  showNeural?: boolean;
  cleanSubtitleStyle?: boolean;
}

const props = defineProps<Props>();

</script>

<style scoped>
  @keyframes float {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(15px); }
  }

  .animate-float {
    animation: float 2s ease-in-out infinite;
  }

  .filter-arrow {
    opacity: 0.25;
    filter:
      drop-shadow(0px 0px 5px rgba(0,0,0,0.5))
      hue-rotate(75deg)
      blur(2.5px);
    transition: all 0.3s ease-in-out;
  }

  @keyframes pulse-slow {
    0%, 100% { opacity: 0.9; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.02); }
  }

  .animate-pulse-slow {
    animation: pulse-slow 5s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  @keyframes gradient-flow {
      0% { background-position: 0% 50%; }
      50% { background-position: 100% 50%; }
      100% { background-position: 0% 50%; }
  }

  .animate-gradient-flow {
      background-size: 200% 200%;
      animation: gradient-flow 6s ease infinite;
      display: inline-block; 
       background-clip: text;
      -webkit-background-clip: text; 
      color: transparent; 
      -webkit-text-fill-color: transparent; 
  }
</style>
