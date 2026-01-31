<template>
  <component
    :is="href ? 'a' : 'button'"
    :href="href"
    :type="type"
    :disabled="disabled"
    :class="[
          'px-6 py-3 text-sm font-bold text-white bg-surface/20 rounded-xl',
          'transition-all duration-300',
          'shadow-[0_0.25rem_0.75rem_rgba(0,0,0,0.2)] hover:shadow-[0_0.5rem_1.5rem_rgba(79,209,197,0.25)]',
          'backdrop-blur-md backdrop-saturate-150 hover:backdrop-saturate-200',
          'border border-white/20 border-solid hover:border-white/30',
          'transform hover:scale-[1.02] hover:-translate-y-0.5 active:scale-[0.98]',
          'relative overflow-hidden group',
          'disabled:opacity-50 disabled:cursor-not-allowed',
          fullWidth ? 'w-full' : '',
          active ? '[&>div:nth-child(2)]:opacity-100' : '',
          className
        ]"
    v-bind="$attrs"
  >
    <!-- Glass effect base -->
    <div class="absolute inset-0 bg-gradient-to-b from-white/10 to-transparent opacity-50"></div>
    
    <!-- Hover/Active glow effect -->
    <div class="absolute inset-0 bg-gradient-to-r from-primary/20 via-primary/10 to-primary/20 opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
    
    <!-- Press effect overlay -->
    <div class="absolute inset-0 bg-black/20 opacity-0 group-active:opacity-100 transition-opacity duration-150"></div>
    
    <!-- Inner shadow -->
    <div class="absolute inset-[0.0625rem] rounded-[0.6875rem] shadow-[inset_0_0.0625rem_0.0625rem_rgba(255,255,255,0.1)]"></div>
    
    <!-- Content -->
    <span class="relative z-10">
      <slot></slot>
    </span>
  </component>
</template>

<script setup lang="ts">
interface Props {
  type?: 'button' | 'submit' | 'reset';
  disabled?: boolean;
  active?: boolean;
  fullWidth?: boolean;
  href?: string;
  className?: string;
}

withDefaults(defineProps<Props>(), {
  type: 'button',
  disabled: false,
  active: false,
  fullWidth: false,
  className: ''
});
</script>