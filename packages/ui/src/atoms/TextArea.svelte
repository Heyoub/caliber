<script lang="ts">
  import { onMount } from 'svelte';
  import type { Size, GlassEffect, BorderEffect } from '../types';

  /**
   * TextArea - Multiline input with auto-resize
   */
  interface Props {
    /** Size variant */
    size?: Size;
    /** Glass morphism effect */
    glass?: GlassEffect;
    /** Border effect */
    border?: BorderEffect;
    /** Error state */
    error?: boolean;
    /** Error message */
    errorMessage?: string;
    /** Disabled state */
    disabled?: boolean;
    /** Readonly state */
    readonly?: boolean;
    /** Value binding */
    value?: string;
    /** Placeholder text */
    placeholder?: string;
    /** Name attribute */
    name?: string;
    /** ID attribute */
    id?: string;
    /** Required field */
    required?: boolean;
    /** Minimum rows */
    rows?: number;
    /** Maximum rows for auto-resize */
    maxRows?: number;
    /** Enable auto-resize */
    autoResize?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Input event handler */
    oninput?: (event: Event & { currentTarget: HTMLTextAreaElement }) => void;
    /** Change event handler */
    onchange?: (event: Event) => void;
    /** Focus event handler */
    onfocus?: (event: FocusEvent) => void;
    /** Blur event handler */
    onblur?: (event: FocusEvent) => void;
  }

  let {
    size = 'md',
    glass = 'medium',
    border = 'subtle',
    error = false,
    errorMessage,
    disabled = false,
    readonly = false,
    value = $bindable(''),
    placeholder,
    name,
    id,
    required = false,
    rows = 3,
    maxRows = 10,
    autoResize = true,
    class: className = '',
    oninput,
    onchange,
    onfocus,
    onblur,
  }: Props = $props();

  // Refs
  let textareaRef: HTMLTextAreaElement;
  let isFocused = $state(false);

  // Size configurations
  const sizeConfigs: Record<Size, { padding: string; text: string }> = {
    xs: { padding: 'p-2', text: 'text-xs' },
    sm: { padding: 'p-2.5', text: 'text-sm' },
    md: { padding: 'p-3', text: 'text-base' },
    lg: { padding: 'p-4', text: 'text-lg' },
    xl: { padding: 'p-5', text: 'text-xl' },
    '2xl': { padding: 'p-6', text: 'text-2xl' },
  };

  // Glass effect classes
  const glassClasses: Record<string, string> = {
    false: 'bg-[hsl(var(--slate-800))]',
    true: 'bg-[hsl(var(--slate-800)/_0.8)] backdrop-blur-md',
    subtle: 'bg-[hsl(var(--slate-800)/_0.5)] backdrop-blur-sm',
    medium: 'bg-[hsl(var(--slate-800)/_0.8)] backdrop-blur-md',
    frosted: 'bg-[hsl(var(--slate-800)/_0.85)] backdrop-blur-xl',
    solid: 'bg-[hsl(var(--slate-800)/_0.9)] backdrop-blur-2xl',
  };

  // Border effect classes
  const borderClasses: Record<string, string> = {
    false: 'border border-[hsl(var(--slate-700))]',
    true: 'border border-[hsl(var(--slate-600))]',
    none: 'border-none',
    subtle: 'border border-[hsl(var(--slate-700)/_0.5)]',
    medium: 'border border-[hsl(var(--slate-600))]',
    strong: 'border-2 border-[hsl(var(--slate-500))]',
    glow: 'border border-[hsl(var(--teal-500)/_0.5)] shadow-[0_0_10px_hsl(var(--teal-500)/_0.2)]',
  };

  let sizeConfig = $derived(sizeConfigs[size]);
  let glassClass = $derived(glassClasses[String(glass)] || glassClasses.medium);
  let borderClass = $derived(borderClasses[String(border)] || borderClasses.subtle);

  // Focus and error styles
  let focusClass = $derived(
    isFocused && !error
      ? 'ring-2 ring-[hsl(var(--teal-500)/_0.5)] border-[hsl(var(--teal-500))]'
      : ''
  );
  let errorClass = $derived(
    error
      ? 'border-[hsl(var(--coral-500))] ring-2 ring-[hsl(var(--coral-500)/_0.3)]'
      : ''
  );

  // Computed classes
  let computedClasses = $derived([
    'w-full',
    'rounded-lg',
    'transition-all duration-200',
    'outline-none resize-none',
    'text-[hsl(var(--slate-100))]',
    'placeholder:text-[hsl(var(--slate-500))]',
    sizeConfig.padding,
    sizeConfig.text,
    glassClass,
    borderClass,
    focusClass,
    errorClass,
    disabled ? 'opacity-50 cursor-not-allowed' : '',
    className,
  ].filter(Boolean).join(' '));

  // Auto-resize function
  function autoResizeTextarea() {
    if (!autoResize || !textareaRef) return;

    // Reset height to auto to get the correct scrollHeight
    textareaRef.style.height = 'auto';

    // Calculate line height from computed styles
    const computedStyle = window.getComputedStyle(textareaRef);
    const lineHeight = parseInt(computedStyle.lineHeight) || 24;
    const paddingTop = parseInt(computedStyle.paddingTop);
    const paddingBottom = parseInt(computedStyle.paddingBottom);

    // Calculate min and max heights
    const minHeight = lineHeight * rows + paddingTop + paddingBottom;
    const maxHeight = lineHeight * maxRows + paddingTop + paddingBottom;

    // Set the new height
    const newHeight = Math.min(Math.max(textareaRef.scrollHeight, minHeight), maxHeight);
    textareaRef.style.height = `${newHeight}px`;
  }

  // Handle input
  function handleInput(e: Event & { currentTarget: HTMLTextAreaElement }) {
    autoResizeTextarea();
    oninput?.(e);
  }

  // Handle focus
  function handleFocus(e: FocusEvent) {
    isFocused = true;
    onfocus?.(e);
  }

  function handleBlur(e: FocusEvent) {
    isFocused = false;
    onblur?.(e);
  }

  // Initialize auto-resize on mount
  onMount(() => {
    if (autoResize && textareaRef) {
      autoResizeTextarea();
    }
  });

  // Re-run auto-resize when value changes
  $effect(() => {
    if (value !== undefined) {
      autoResizeTextarea();
    }
  });
</script>

<div class="flex flex-col gap-1">
  <textarea
    bind:this={textareaRef}
    {name}
    {id}
    {placeholder}
    {required}
    {disabled}
    {readonly}
    {rows}
    bind:value
    class={computedClasses}
    aria-invalid={error}
    aria-describedby={error && errorMessage ? `${id || name}-error` : undefined}
    onfocus={handleFocus}
    onblur={handleBlur}
    oninput={handleInput}
    {onchange}
  ></textarea>

  {#if error && errorMessage}
    <span
      id={id || name ? `${id || name}-error` : undefined}
      class="text-xs text-[hsl(var(--coral-400))] pl-1"
    >
      {errorMessage}
    </span>
  {/if}
</div>

<style>
  textarea:focus {
    outline: none;
  }

  /* Custom scrollbar */
  textarea::-webkit-scrollbar {
    width: 6px;
  }
  textarea::-webkit-scrollbar-track {
    background: transparent;
  }
  textarea::-webkit-scrollbar-thumb {
    background: hsl(var(--slate-600));
    border-radius: 3px;
  }
  textarea::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--slate-500));
  }
</style>
