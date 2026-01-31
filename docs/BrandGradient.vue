<template>
  <component :is="props.as" v-bind="$attrs">
    <slot />
  </component>
</template>

<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  as?: string; // HTML tag name
}

const props = withDefaults(defineProps<Props>(), {
  as: 'div', // Default to div if not specified
});

// No specific logic needed here, as styling and content are passed via attrs/slots
</script>

<style scoped>
/* 
  No specific styles needed here. 
  The expectation is that gradient styles (like background-clip: text) 
  are applied via classes passed to this component from its parent, 
  e.g., using Tailwind utilities or global CSS associated with passed classes. 
*/
</style>
