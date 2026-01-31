<template>
  <span class="basic-badge">
    <slot>{{ text }}</slot>
    <!-- TODO: Restore full Badge.vue functionality and styling. 
         This is a temporary minimal version for testing purposes due to mount/slot issues. -->
  </span>
</template>

<script setup lang="ts">
defineProps({
  text: {
    type: String,
    default: 'Badge',
  },
  // Add other critical props if absolutely necessary for other parts of the app to not break,
  // but keep it minimal. For now, only 'text' and default slot.
});
</script>

<style scoped>
.basic-badge {
  display: inline-block;
  padding: 0.25em 0.5em;
  border-radius: 0.25rem;
  background-color: #eee;
  color: #333;
  font-size: 0.875rem;
  border: 1px solid #ccc;
}
</style>