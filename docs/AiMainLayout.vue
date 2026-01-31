<script setup lang="ts">
import { defineProps } from 'vue';

defineProps<{ showContext?: boolean }>();
</script>

<template>
  <div class="flex h-screen w-screen bg-slate-950 text-slate-100">
    <!-- sidebar slot -->
    <slot name="sidebar" />
    <!-- main -->
    <main class="flex-1 overflow-y-auto bg-slate-900">
      <RouterView />
    </main>
    <!-- (optional) context panel -->
    <slot name="context" v-if="showContext" />
  </div>
</template>