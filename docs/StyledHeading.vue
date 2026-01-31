<template>
  <h1 class="styled-heading">
    <slot />
  </h1>
</template>

<script setup lang="ts">
// Add any props or logic here if needed
</script>

<style scoped>
.styled-heading {
  font-size: 2.5rem;
  font-weight: bold;
  color: #34d399;
  margin-bottom: 1.5rem;
}
</style>
