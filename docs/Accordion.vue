<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';

defineProps<{
  title: string;
  preview?: string;
}>();

const isOpen = ref(false);
const accordionRef = ref<HTMLElement | null>(null);

const handleClickOutside = (event: MouseEvent) => {
  if (accordionRef.value && !accordionRef.value.contains(event.target as Node)) {
    isOpen.value = false;
  }
};

onMounted(() => {
  document.addEventListener('click', handleClickOutside);
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
});
</script>

<template>
  <div class="mb-2 w-[90%] mx-auto" ref="accordionRef">
    <div 
      class="rounded-lg bg-transparent border border-surface/40 transition-all duration-200 ease-out overflow-hidden glass-card"
      :class="{ 
        'shadow-[inset_0_-1px_2px_rgba(30,41,59,0.4)] hover:shadow-[inset_0_-1px_4px_rgba(30,41,59,0.5)]': !isOpen,
        'shadow-[inset_0_-2px_8px_rgba(30,41,59,0.6)]': isOpen
      }"
    >
      <button 
        @click.stop="isOpen = !isOpen"
        class="w-full text-left py-1.5 px-2 relative group"
      >
        <div class="flex items-center justify-between">
          <div class="flex-1 pr-4">
            <div v-if="preview && !isOpen" class="text-base text-gray-300/90 leading-relaxed pr-8">
              {{ preview.split(' and innovate')[0] + ' and...' }}
            </div>
            <div v-else>
              <slot name="title" />
            </div>
          </div>
          <div 
            class="w-5 h-5 flex items-center justify-center transition-transform duration-200 ease-out"
            :class="{ 'rotate-180': isOpen }"
          >
            <svg 
              xmlns="http://www.w3.org/2000/svg" 
              class="w-4 h-4 text-primary/60 group-hover:text-primary transition-colors duration-200" 
              fill="none" 
              viewBox="0 0 24 24" 
              stroke="currentColor"
            >
              <path 
                stroke-linecap="round" 
                stroke-linejoin="round" 
                stroke-width="2" 
                d="M19 9l-7 7-7-7" 
              />
            </svg>
          </div>
        </div>
      </button>
      <div 
        class="transition-all duration-150 ease-in-out"
        :style="{ 
          maxHeight: isOpen ? '1000px' : '0',
          opacity: isOpen ? '1' : '0',
          transform: isOpen ? 'translateY(0)' : 'translateY(-2px)'
        }"
      >
        <div class="px-2 pb-2">
          <div class="text-base text-gray-300 leading-relaxed inverted-triangle">
            <slot />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.rotate-180 {
  transform: rotate(180deg);
}

.inverted-triangle {
  text-align: justify;
  max-width: 95%;
  margin: 0 auto;
  padding: 0.25em 0;
}

.inverted-triangle :deep(p) {
  margin: 0;
  padding: 0.25em 0;
  line-height: 1.6;
}

.inverted-triangle :deep(p:first-of-type) {
  max-width: 75%;
  margin: 0 auto;
}

.inverted-triangle :deep(p:nth-of-type(2)) {
  max-width: 85%;
  margin: 0 auto;
}

.inverted-triangle :deep(p:nth-of-type(3)) {
  max-width: 90%;
  margin: 0 auto;
}

.inverted-triangle :deep(p:nth-of-type(n+4)) {
  max-width: 95%;
  margin: 0 auto;
}
</style>
