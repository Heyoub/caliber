<script lang="ts">
  /**
   * Modal - Dialog overlay with glass effect backdrop
   * Features focus trap and escape to close
   */
  import type { Snippet } from 'svelte';
  import { onMount, onDestroy, tick } from 'svelte';

  type Size = 'sm' | 'md' | 'lg' | 'xl' | 'full';
  type GlassEffect = 'subtle' | 'medium' | 'frosted' | 'solid';

  interface Props {
    /** Open state */
    open?: boolean;
    /** Modal size */
    size?: Size;
    /** Glass effect for modal panel */
    glass?: GlassEffect;
    /** Show close button */
    showClose?: boolean;
    /** Close on backdrop click */
    closeOnBackdrop?: boolean;
    /** Close on escape key */
    closeOnEscape?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Modal title */
    title?: string;
    /** Header slot */
    header?: Snippet;
    /** Default slot for content */
    children: Snippet;
    /** Footer slot */
    footer?: Snippet;
    /** Close handler */
    onclose?: () => void;
  }

  let {
    open = $bindable(false),
    size = 'md',
    glass = 'frosted',
    showClose = true,
    closeOnBackdrop = true,
    closeOnEscape = true,
    class: className = '',
    title = '',
    header,
    children,
    footer,
    onclose
  }: Props = $props();

  let modalRef: HTMLElement;
  let previousActiveElement: HTMLElement | null = null;
  let focusableElements: HTMLElement[] = [];

  // Size mappings
  const sizeClasses: Record<Size, string> = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg',
    xl: 'max-w-xl',
    full: 'max-w-[90vw] max-h-[90vh]'
  };

  // Glass effect mappings
  const glassClasses: Record<GlassEffect, string> = {
    subtle: 'backdrop-blur-sm bg-slate-800/80',
    medium: 'backdrop-blur-md bg-slate-800/85',
    frosted: 'backdrop-blur-xl bg-slate-800/90',
    solid: 'backdrop-blur-2xl bg-slate-800/95'
  };

  function closeModal() {
    open = false;
    onclose?.();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (closeOnBackdrop && e.target === e.currentTarget) {
      closeModal();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open) return;

    if (e.key === 'Escape' && closeOnEscape) {
      e.preventDefault();
      closeModal();
      return;
    }

    // Focus trap
    if (e.key === 'Tab') {
      if (focusableElements.length === 0) {
        e.preventDefault();
        return;
      }

      const firstElement = focusableElements[0];
      const lastElement = focusableElements[focusableElements.length - 1];

      if (e.shiftKey && document.activeElement === firstElement) {
        e.preventDefault();
        lastElement.focus();
      } else if (!e.shiftKey && document.activeElement === lastElement) {
        e.preventDefault();
        firstElement.focus();
      }
    }
  }

  function updateFocusableElements() {
    if (!modalRef) return;

    const selectors = [
      'button:not([disabled])',
      'input:not([disabled])',
      'select:not([disabled])',
      'textarea:not([disabled])',
      'a[href]',
      '[tabindex]:not([tabindex="-1"])'
    ];

    focusableElements = Array.from(
      modalRef.querySelectorAll<HTMLElement>(selectors.join(','))
    );
  }

  // Handle open state changes
  $effect(() => {
    if (open) {
      previousActiveElement = document.activeElement as HTMLElement;
      document.body.style.overflow = 'hidden';

      tick().then(() => {
        updateFocusableElements();
        // Focus first focusable element or modal itself
        if (focusableElements.length > 0) {
          focusableElements[0].focus();
        } else {
          modalRef?.focus();
        }
      });
    } else {
      document.body.style.overflow = '';
      previousActiveElement?.focus();
    }
  });

  onMount(() => {
    document.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeydown);
    document.body.style.overflow = '';
  });
</script>

{#if open}
  <!-- Backdrop -->
  <div
    class="modal-backdrop fixed inset-0 z-50 flex items-center justify-center p-4
           bg-slate-950/60 backdrop-blur-sm"
    onclick={handleBackdropClick}
    role="dialog"
    aria-modal="true"
    aria-labelledby={title ? 'modal-title' : undefined}
  >
    <!-- Modal Panel -->
    <div
      bind:this={modalRef}
      class={`
        modal-panel w-full rounded-xl border border-slate-600/30
        shadow-[0_8px_32px_rgba(0,0,0,0.4)]
        ${sizeClasses[size]}
        ${glassClasses[glass]}
        ${className}
      `}
      tabindex="-1"
    >
      <!-- Header -->
      {#if header || title || showClose}
        <div class="flex items-center justify-between px-6 py-4 border-b border-slate-600/30">
          {#if header}
            {@render header()}
          {:else if title}
            <h2 id="modal-title" class="text-lg font-semibold text-white">
              {title}
            </h2>
          {/if}

          {#if showClose}
            <button
              type="button"
              onclick={closeModal}
              class="p-2 -m-2 text-slate-400 hover:text-white transition-colors rounded-lg
                     hover:bg-slate-700/50 focus:outline-none focus:ring-2 focus:ring-teal-500/50"
              aria-label="Close modal"
            >
              <svg
                class="w-5 h-5"
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
        </div>
      {/if}

      <!-- Content -->
      <div class="px-6 py-4 text-slate-200 overflow-y-auto max-h-[60vh]">
        {@render children()}
      </div>

      <!-- Footer -->
      {#if footer}
        <div class="px-6 py-4 border-t border-slate-600/30 flex items-center justify-end gap-3">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    animation: backdrop-in 0.2s ease-out;
  }

  .modal-panel {
    animation: modal-in 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  @keyframes backdrop-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes modal-in {
    from {
      opacity: 0;
      transform: scale(0.95) translateY(-10px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }
</style>
