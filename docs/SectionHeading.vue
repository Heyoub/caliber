<template>
  <div :class="['mb-8 xs:mb-10 sm:mb-12', alignmentClass]">
    <h2 class="text-2xl xs:text-3xl sm:text-4xl font-bold mb-2 xs:mb-3 sm:mb-4 bg-gradient-to-r from-primary to-purple-600 bg-clip-text text-transparent">
      {{ title }}
    </h2>
    <p v-if="subtitle" class="text-base xs:text-lg sm:text-xl text-gray-400 max-w-3xl mx-auto">
      {{ subtitle }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { computed, defineProps } from 'vue';

type Alignment = 'left' | 'center' | 'right';

const props = defineProps({
  title: { type: String, required: true },
  subtitle: { type: String, default: '' },
  alignment: {
    type: String as () => Alignment,
    default: 'center',
    validator: (value: Alignment) => ['left', 'center', 'right'].includes(value)
  }
});

const alignmentClass = computed(() => {
  switch (props.alignment) {
    case 'left': return 'text-left';
    case 'right': return 'text-right';
    case 'center':
    default:
      return 'text-center';
  }
});
</script>

<style scoped>
/* Scoped styles if needed, though most styling is Tailwind */
.from-primary {
  --tw-gradient-from: #4FD1C5;
  --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, rgba(79, 209, 197, 0));
}
.to-purple-600 {
    --tw-gradient-to: #805AD5;
}
</style>
