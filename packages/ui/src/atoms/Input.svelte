<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Size, GlassEffect, BorderEffect } from '../types';

  /**
   * Input - Text input with glass effect and slots
   */
  interface Props {
    /** Input type */
    type?: 'text' | 'email' | 'password' | 'search' | 'tel' | 'url' | 'number';
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
    /** Autofocus */
    autofocus?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Input event handler */
    oninput?: (event: Event & { currentTarget: HTMLInputElement }) => void;
    /** Change event handler */
    onchange?: (event: Event) => void;
    /** Focus event handler */
    onfocus?: (event: FocusEvent) => void;
    /** Blur event handler */
    onblur?: (event: FocusEvent) => void;
    /** Prefix slot content */
    prefix?: Snippet;
    /** Suffix slot content */
    suffix?: Snippet;
  }

  let {
    type = 'text',
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
    autofocus = false,
    class: className = '',
    oninput,
    onchange,
    onfocus,
    onblur,
    prefix,
    suffix,
  }: Props = $props();

  // Reactive state
  let isFocused = $state(false);

  // Size configurations
  const sizeConfigs: Record<Size, { input: string; text: string; icon: string }> = {
    xs: { input: 'h-7 px-2', text: 'text-xs', icon: 'w-3 h-3' },
    sm: { input: 'h-8 px-2.5', text: 'text-sm', icon: 'w-4 h-4' },
    md: { input: 'h-10 px-3', text: 'text-base', icon: 'w-5 h-5' },
    lg: { input: 'h-12 px-4', text: 'text-lg', icon: 'w-5 h-5' },
    xl: { input: 'h-14 px-5', text: 'text-xl', icon: 'w-6 h-6' },
    '2xl': { input: 'h-16 px-6', text: 'text-2xl', icon: 'w-7 h-7' },
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

  // Computed wrapper classes
  let wrapperClasses = $derived([
    'relative flex items-center',
    'rounded-lg',
    'transition-all duration-200',
    sizeConfig.input,
    glassClass,
    borderClass,
    focusClass,
    errorClass,
    disabled ? 'opacity-50 cursor-not-allowed' : '',
    className,
  ].filter(Boolean).join(' '));

  // Handle focus
  function handleFocus(e: FocusEvent) {
    isFocused = true;
    onfocus?.(e);
  }

  function handleBlur(e: FocusEvent) {
    isFocused = false;
    onblur?.(e);
  }
</script>

<div class="flex flex-col gap-1">
  <div class={wrapperClasses}>
    {#if prefix}
      <span class="flex-shrink-0 text-[hsl(var(--slate-400))] mr-2">
        {@render prefix()}
      </span>
    {/if}

    <input
      {type}
      {name}
      {id}
      {placeholder}
      {required}
      {autofocus}
      {disabled}
      {readonly}
      bind:value
      class="flex-1 bg-transparent outline-none text-[hsl(var(--slate-100))] placeholder:text-[hsl(var(--slate-500))] {sizeConfig.text} disabled:cursor-not-allowed"
      aria-invalid={error}
      aria-describedby={error && errorMessage ? `${id || name}-error` : undefined}
      onfocus={handleFocus}
      onblur={handleBlur}
      {oninput}
      {onchange}
    />

    {#if suffix}
      <span class="flex-shrink-0 text-[hsl(var(--slate-400))] ml-2">
        {@render suffix()}
      </span>
    {/if}
  </div>

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
  /* Focus glow effect */
  input:focus {
    outline: none;
  }

  /* Autofill styling override */
  input:-webkit-autofill,
  input:-webkit-autofill:hover,
  input:-webkit-autofill:focus {
    -webkit-text-fill-color: hsl(var(--slate-100));
    -webkit-box-shadow: 0 0 0px 1000px hsl(var(--slate-800)) inset;
    transition: background-color 5000s ease-in-out 0s;
  }
</style>
