<template>
  <PathProvider>
    <router-view /> <!-- Router now controls the top-level layout -->
  </PathProvider>
</template>

<script setup lang="ts">
// Font imports
import '@fontsource/space-grotesk/400.css'; // Regular weight
import '@fontsource/space-grotesk/500.css'; // Medium weight
import '@fontsource/space-grotesk/700.css'; // Bold weight

// Import Base Styles and Fonts
import './styles/fonts.css';
import './styles/scroll.css';
import './styles/global.css';
import './styles/matter.css';
import './styles/design-system.css';

// Component imports
// MatterLayout will be imported and used in router.ts for specific routes
import PathProvider from './ai/store/PathProvider.vue';
</script>

<style scoped>
/* Remove previous styles if any */
</style>
