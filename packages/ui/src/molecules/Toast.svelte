<script lang="ts">
  /**
   * Toast - Notification toast with auto-dismiss
   * Supports action button and multiple variants
   */
  import type { Snippet } from 'svelte';
  import { onMount, onDestroy } from 'svelte';

  type ToastVariant = 'info' | 'success' | 'warning' | 'error';
  type Position = 'top-left' | 'top-center' | 'top-right' |
                  'bottom-left' | 'bottom-center' | 'bottom-right';

  interface Props {
    /** Toast message */
    message: string;
    /** Toast variant */
    variant?: ToastVariant;
    /** Auto-dismiss duration in ms (0 to disable) */
    duration?: number;
    /** Show close button */
    showClose?: boolean;
    /** Action button label */
    actionLabel?: string;
    /** Icon snippet */
    icon?: Snippet;
    /** Visible state */
    visible?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Close handler */
    onclose?: () => void;
    /** Action click handler */
    onaction?: () => void;
  }

  let {
    message,
    variant = 'info',
    duration = 5000,
    showClose = true,
    actionLabel = '',
    icon,
    visible = $bindable(true),
    class: className = '',
    onclose,
    onaction
  }: Props = $props();

  let dismissTimer: ReturnType<typeof setTimeout>;
  let progressWidth = $state(100);
  let progressInterval: ReturnType<typeof setInterval>;

  // Variant styles
  const variantClasses: Record<ToastVariant, { bg: string; border: string; icon: string }> = {
    info: {
      bg: 'bg-slate-800/90',
      border: 'border-teal-500/30',
      icon: 'text-teal-400'
    },
    success: {
      bg: 'bg-slate-800/90',
      border: 'border-mint-500/30',
      icon: 'text-mint-400'
    },
    warning: {
      bg: 'bg-slate-800/90',
      border: 'border-amber-500/30',
      icon: 'text-amber-400'
    },
    error: {
      bg: 'bg-slate-800/90',
      border: 'border-coral-500/30',
      icon: 'text-coral-400'
    }
  };

  // Progress bar colors
  const progressColors: Record<ToastVariant, string> = {
    info: 'bg-teal-500',
    success: 'bg-mint-500',
    warning: 'bg-amber-500',
    error: 'bg-coral-500'
  };

  // Default icons
  const defaultIcons: Record<ToastVariant, string> = {
    info: 'M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z',
    success: 'M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z',
    warning: 'M1 21h22L12 2 1 21zm12-3h-2v-2h2v2zm0-4h-2v-4h2v4z',
    error: 'M12 2C6.47 2 2 6.47 2 12s4.47 10 10 10 10-4.47 10-10S17.53 2 12 2zm5 13.59L15.59 17 12 13.41 8.41 17 7 15.59 10.59 12 7 8.41 8.41 7 12 10.59 15.59 7 17 8.41 13.41 12 17 15.59z'
  };

  function dismiss() {
    visible = false;
    onclose?.();
    clearTimeout(dismissTimer);
    clearInterval(progressInterval);
  }

  function handleAction() {
    onaction?.();
    dismiss();
  }

  function startAutoDismiss() {
    if (duration <= 0) return;

    // Progress animation
    const step = 100 / (duration / 50);
    progressInterval = setInterval(() => {
      progressWidth = Math.max(0, progressWidth - step);
    }, 50);

    // Dismiss timer
    dismissTimer = setTimeout(dismiss, duration);
  }

  function pauseAutoDismiss() {
    clearTimeout(dismissTimer);
    clearInterval(progressInterval);
  }

  function resumeAutoDismiss() {
    if (duration <= 0 || progressWidth <= 0) return;

    const remainingTime = (progressWidth / 100) * duration;
    const step = progressWidth / (remainingTime / 50);

    progressInterval = setInterval(() => {
      progressWidth = Math.max(0, progressWidth - step);
    }, 50);

    dismissTimer = setTimeout(dismiss, remainingTime);
  }

  onMount(() => {
    if (visible && duration > 0) {
      startAutoDismiss();
    }
  });

  onDestroy(() => {
    clearTimeout(dismissTimer);
    clearInterval(progressInterval);
  });

  // Restart timer when visibility changes
  $effect(() => {
    if (visible && duration > 0) {
      progressWidth = 100;
      startAutoDismiss();
    }
  });

  const styles = $derived(variantClasses[variant]);
</script>

{#if visible}
  <div
    class={`
      toast flex items-start gap-3 w-full max-w-sm p-4
      backdrop-blur-xl rounded-lg border
      shadow-[0_4px_20px_rgba(0,0,0,0.3)]
      ${styles.bg}
      ${styles.border}
      ${className}
    `}
    role="alert"
    aria-live="polite"
    onmouseenter={pauseAutoDismiss}
    onmouseleave={resumeAutoDismiss}
  >
    <!-- Icon -->
    <div class={`flex-shrink-0 ${styles.icon}`}>
      {#if icon}
        {@render icon()}
      {:else}
        <svg
          class="w-5 h-5"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d={defaultIcons[variant]} />
        </svg>
      {/if}
    </div>

    <!-- Content -->
    <div class="flex-1 min-w-0">
      <p class="text-sm text-slate-200">{message}</p>

      {#if actionLabel}
        <button
          type="button"
          onclick={handleAction}
          class="mt-2 text-sm font-medium text-teal-400 hover:text-teal-300
                 focus:outline-none focus:underline transition-colors"
        >
          {actionLabel}
        </button>
      {/if}
    </div>

    <!-- Close button -->
    {#if showClose}
      <button
        type="button"
        onclick={dismiss}
        class="flex-shrink-0 p-1 -m-1 text-slate-400 hover:text-white
               transition-colors focus:outline-none focus:ring-2
               focus:ring-teal-500/50 rounded"
        aria-label="Dismiss"
      >
        <svg
          class="w-4 h-4"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M18 6 6 18" />
          <path d="m6 6 12 12" />
        </svg>
      </button>
    {/if}

    <!-- Progress bar -->
    {#if duration > 0}
      <div class="absolute bottom-0 left-0 right-0 h-1 bg-slate-700/50 rounded-b-lg overflow-hidden">
        <div
          class={`h-full transition-all duration-50 ${progressColors[variant]}`}
          style={`width: ${progressWidth}%`}
        ></div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .toast {
    position: relative;
    animation: toast-in 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(-10px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
</style>
