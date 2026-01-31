<script lang="ts">
  /**
   * InputGroup - Input with prefix/suffix addons
   * Combines input atom with optional prefix and suffix slots
   */
  import type { Snippet } from 'svelte';

  type Size = 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  type GlassEffect = boolean | 'subtle' | 'medium' | 'frosted' | 'solid';
  type BorderEffect = boolean | 'none' | 'subtle' | 'medium' | 'strong' | 'glow';

  interface Props {
    /** Input value */
    value?: string;
    /** Input type */
    type?: 'text' | 'email' | 'password' | 'number' | 'tel' | 'url';
    /** Placeholder text */
    placeholder?: string;
    /** Label text */
    label?: string;
    /** Component size */
    size?: Size;
    /** Glass effect variant */
    glass?: GlassEffect;
    /** Border effect variant */
    border?: BorderEffect;
    /** Error state */
    error?: boolean | string;
    /** Disabled state */
    disabled?: boolean;
    /** Required field */
    required?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Prefix content */
    prefix?: Snippet;
    /** Suffix content */
    suffix?: Snippet;
    /** Input event handler */
    oninput?: (event: Event) => void;
    /** Change event handler */
    onchange?: (event: Event) => void;
    /** Focus event handler */
    onfocus?: (event: FocusEvent) => void;
    /** Blur event handler */
    onblur?: (event: FocusEvent) => void;
  }

  let {
    value = $bindable(''),
    type = 'text',
    placeholder = '',
    label = '',
    size = 'md',
    glass = 'medium',
    border = 'subtle',
    error = false,
    disabled = false,
    required = false,
    class: className = '',
    prefix,
    suffix,
    oninput,
    onchange,
    onfocus,
    onblur
  }: Props = $props();

  // Size mappings
  const sizeClasses: Record<Size, { container: string; input: string; addon: string }> = {
    xs: { container: 'h-7', input: 'text-xs px-2', addon: 'px-2 text-xs' },
    sm: { container: 'h-8', input: 'text-sm px-2.5', addon: 'px-2.5 text-sm' },
    md: { container: 'h-10', input: 'text-base px-3', addon: 'px-3 text-base' },
    lg: { container: 'h-12', input: 'text-lg px-4', addon: 'px-4 text-lg' },
    xl: { container: 'h-14', input: 'text-xl px-5', addon: 'px-5 text-xl' }
  };

  // Glass effect mappings
  const glassClasses: Record<string, string> = {
    'true': 'backdrop-blur-md bg-slate-800/60',
    'false': 'bg-slate-800',
    'subtle': 'backdrop-blur-sm bg-slate-800/40',
    'medium': 'backdrop-blur-md bg-slate-800/60',
    'frosted': 'backdrop-blur-xl bg-slate-800/70',
    'solid': 'backdrop-blur-2xl bg-slate-800/80'
  };

  // Border effect mappings
  const borderClasses: Record<string, string> = {
    'true': 'border border-slate-600/50',
    'false': 'border-0',
    'none': 'border-0',
    'subtle': 'border border-slate-600/30',
    'medium': 'border border-slate-600/50',
    'strong': 'border-2 border-slate-500/60',
    'glow': 'border border-teal-500/40 shadow-[0_0_10px_hsl(176_55%_45%/0.15)]'
  };

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    value = target.value;
    oninput?.(event);
  }

  // Computed classes
  const containerClasses = $derived(
    `input-group flex items-stretch rounded-lg overflow-hidden transition-all duration-200
    ${sizeClasses[size].container}
    ${glassClasses[String(glass)]}
    ${borderClasses[String(border)]}
    ${error ? 'border-coral-500/50 focus-within:border-coral-500' : ''}
    ${disabled ? 'opacity-50 pointer-events-none' : ''}
    ${className}`.trim()
  );

  const addonClasses = $derived(
    `flex items-center bg-slate-700/50 text-slate-400 border-slate-600/30
    ${sizeClasses[size].addon}`.trim()
  );

  const inputClasses = $derived(
    `flex-1 bg-transparent border-0 outline-none text-slate-100 placeholder-slate-500
    focus:ring-0 focus:outline-none
    ${sizeClasses[size].input}`.trim()
  );
</script>

<div class="input-group-wrapper">
  {#if label}
    <label class="block text-sm font-medium text-slate-300 mb-1.5">
      {label}
      {#if required}
        <span class="text-coral-400 ml-0.5">*</span>
      {/if}
    </label>
  {/if}

  <div class={containerClasses}>
    <!-- Prefix addon -->
    {#if prefix}
      <div class={`${addonClasses} border-r`}>
        {@render prefix()}
      </div>
    {/if}

    <!-- Input -->
    <input
      {type}
      {value}
      {placeholder}
      {disabled}
      {required}
      oninput={handleInput}
      {onchange}
      {onfocus}
      {onblur}
      class={inputClasses}
      aria-invalid={!!error}
      aria-describedby={error && typeof error === 'string' ? 'input-error' : undefined}
    />

    <!-- Suffix addon -->
    {#if suffix}
      <div class={`${addonClasses} border-l`}>
        {@render suffix()}
      </div>
    {/if}
  </div>

  <!-- Error message -->
  {#if error && typeof error === 'string'}
    <p id="input-error" class="mt-1.5 text-sm text-coral-400">
      {error}
    </p>
  {/if}
</div>

<style>
  .input-group:focus-within {
    box-shadow: 0 0 0 2px hsl(176 55% 45% / 0.2);
  }

  .input-group:focus-within:has([aria-invalid="true"]) {
    box-shadow: 0 0 0 2px hsl(12 65% 50% / 0.2);
  }
</style>
