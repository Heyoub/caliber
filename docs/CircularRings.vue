<template>
  <div class="circular-rings p-24">
    <div class="ring ring1"></div>
    <div class="ring ring2"></div>
    <div class="ring ring3"></div>
    <div class="ring ring4"></div>
  </div>
</template>

<script setup lang="ts">
// No logic required
</script>

<style scoped>
.circular-rings {
  position: relative;
  width: 120px;
  height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.ring {
  position: absolute;
  border-radius: 50%;
  border: 2px solid rgba(45,212,191,0.25);
  animation: pulse-ring 2.5s cubic-bezier(0.4, 0, 0.2, 1) infinite;
}
.ring1 {
  width: 120px;
  height: 120px;
  border-color: rgba(45,212,191,0.25);
  animation-delay: 0s;
}
.ring2 {
  width: 90px;
  height: 90px;
  border-color: rgba(110,231,183,0.25);
  animation-delay: 0.6s;
}
.ring3 {
  width: 60px;
  height: 60px;
  border-color: rgba(167,139,250,0.25);
  animation-delay: 1.2s;
}
.ring4 {
  width: 30px;
  height: 30px;
  border-color: rgba(244,63,94,0.25);
  animation-delay: 1.8s;
}
@keyframes pulse-ring {
  0%, 100% {
    opacity: 0.8;
    transform: scale(1);
  }
  50% {
    opacity: 0.2;
    transform: scale(1.08);
  }
}
</style>