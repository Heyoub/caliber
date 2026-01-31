<!--
  ToolCallCard.svelte - MCP tool execution display
  Based on ARCHITECTURE.md MCP UI Patterns

  Features:
  - Status badge (pending/running/success/error)
  - Arguments display (TreeView compatible)
  - Result display
  - Approve/Reject buttons for pending
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { ToolCall, ToolResult, ToolCallStatusLiteral, ColorPalette, CMSContent } from '../types/index.js';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Tool call data */
    call: ToolCall;
    /** Tool result (if available) */
    result?: ToolResult;
    /** Expanded state */
    expanded?: boolean;
    /** Show arguments by default */
    showArguments?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Arguments display slot */
    argumentsSlot?: Snippet;
    /** Result display slot */
    resultSlot?: Snippet;
    /** Event handlers */
    onApprove?: () => void;
    onReject?: () => void;
    onToggle?: () => void;
  }

  let {
    cms = {},
    call,
    result,
    expanded = false,
    showArguments = true,
    class: className = '',
    argumentsSlot,
    resultSlot,
    onApprove,
    onReject,
    onToggle
  }: Props = $props();

  // Status to color mapping
  const statusColors: Record<ToolCallStatusLiteral, ColorPalette> = {
    pending: 'amber',
    approved: 'teal',
    running: 'purple',
    success: 'mint',
    error: 'coral',
    rejected: 'coral'
  };

  const statusLabels: Record<ToolCallStatusLiteral, string> = {
    pending: cms.pendingLabel || 'Pending',
    approved: cms.approvedLabel || 'Approved',
    running: cms.runningLabel || 'Running',
    success: cms.successLabel || 'Success',
    error: cms.errorLabel || 'Error',
    rejected: cms.errorLabel || 'Rejected'
  };

  // Derived values
  const color = $derived(statusColors[call.status] || 'slate');
  const isPending = $derived(call.status === 'pending');
  const isRunning = $derived(call.status === 'running');
  const isComplete = $derived(call.status === 'success' || call.status === 'error');

  // Card classes based on status
  const cardClasses = $derived(`
    relative overflow-hidden rounded-xl backdrop-blur-md transition-all duration-300
    bg-slate-800/50 border border-solid
    ${color === 'amber' ? 'border-amber-500/30 hover:border-amber-500/50' : ''}
    ${color === 'teal' ? 'border-teal-500/30 hover:border-teal-500/50' : ''}
    ${color === 'purple' ? 'border-purple-500/30 hover:border-purple-500/50' : ''}
    ${color === 'mint' ? 'border-mint-500/30 hover:border-mint-500/50' : ''}
    ${color === 'coral' ? 'border-coral-500/30 hover:border-coral-500/50' : ''}
    ${className}
  `);

  // Badge classes
  const badgeClasses = $derived(`
    inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium
    ${color === 'amber' ? 'bg-amber-500/20 text-amber-300' : ''}
    ${color === 'teal' ? 'bg-teal-500/20 text-teal-300' : ''}
    ${color === 'purple' ? 'bg-purple-500/20 text-purple-300' : ''}
    ${color === 'mint' ? 'bg-mint-500/20 text-mint-300' : ''}
    ${color === 'coral' ? 'bg-coral-500/20 text-coral-300' : ''}
  `);

  // Format arguments for display
  function formatArguments(args: Record<string, unknown>): string {
    return JSON.stringify(args, null, 2);
  }

  // Format duration
  const formattedDuration = $derived(
    call.duration ? `${call.duration}ms` : ''
  );
</script>

<div class={cardClasses}>
  <!-- Status indicator bar -->
  <div
    class="absolute top-0 left-0 right-0 h-1"
    class:bg-amber-500={color === 'amber'}
    class:bg-teal-500={color === 'teal'}
    class:bg-purple-500={color === 'purple'}
    class:bg-mint-500={color === 'mint'}
    class:bg-coral-500={color === 'coral'}
    class:animate-pulse={isRunning}
  ></div>

  <!-- Header -->
  <header class="flex items-center justify-between px-4 py-3 border-b border-slate-700/50">
    <div class="flex items-center gap-3">
      <!-- Status badge -->
      <span class={badgeClasses}>
        <!-- Tool icon -->
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
        {call.name}
      </span>

      <!-- Status label -->
      <span class="text-xs text-slate-400">
        {statusLabels[call.status]}
      </span>

      <!-- Running spinner -->
      {#if isRunning}
        <svg class="w-4 h-4 text-purple-400 animate-spin" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
      {/if}
    </div>

    <div class="flex items-center gap-3">
      <!-- Duration -->
      {#if formattedDuration}
        <span class="text-xs text-slate-500">{formattedDuration}</span>
      {/if}

      <!-- Expand/collapse button -->
      <button
        class="p-1 rounded hover:bg-slate-700/50 transition-colors"
        onclick={() => onToggle?.()}
      >
        <svg
          class="w-4 h-4 text-slate-400 transition-transform duration-200"
          class:rotate-180={expanded}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </button>
    </div>
  </header>

  <!-- Expandable content -->
  {#if expanded}
    <div class="px-4 py-3 space-y-4">
      <!-- Arguments -->
      {#if showArguments && Object.keys(call.arguments).length > 0}
        <div>
          <div class="text-xs font-medium text-slate-400 mb-2">
            {cms.argumentsLabel || 'Arguments'}
          </div>
          {#if argumentsSlot}
            {@render argumentsSlot()}
          {:else}
            <pre class="text-xs text-slate-300 bg-slate-900/50 rounded-lg p-3 overflow-x-auto font-mono">{formatArguments(call.arguments)}</pre>
          {/if}
        </div>
      {/if}

      <!-- Result -->
      {#if result}
        <div class="pt-3 border-t border-slate-700/50">
          <div class="text-xs font-medium text-slate-400 mb-2">
            {cms.resultLabel || 'Result'}
          </div>
          {#if resultSlot}
            {@render resultSlot()}
          {:else if result.error}
            <div class="text-sm text-coral-400 bg-coral-500/10 rounded-lg p-3 border border-coral-500/20">
              {result.error}
            </div>
          {:else if result.data}
            <pre class="text-xs text-slate-300 bg-slate-900/50 rounded-lg p-3 overflow-x-auto font-mono">{JSON.stringify(result.data, null, 2)}</pre>
          {:else}
            <div class="text-sm text-mint-400">
              {cms.successMessage || 'Completed successfully'}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Approval buttons (for pending status) -->
  {#if isPending}
    <footer class="flex items-center justify-end gap-2 px-4 py-3 border-t border-slate-700/50 bg-slate-800/30">
      <button
        class="px-3 py-1.5 text-sm font-medium text-slate-300 hover:text-white bg-slate-700/50 hover:bg-slate-600/50 rounded-lg transition-colors"
        onclick={() => onReject?.()}
      >
        {cms.rejectLabel || 'Reject'}
      </button>
      <button
        class="px-3 py-1.5 text-sm font-medium text-white bg-teal-500 hover:bg-teal-400 rounded-lg transition-colors shadow-lg shadow-teal-500/25"
        onclick={() => onApprove?.()}
      >
        {cms.approveLabel || 'Approve'}
      </button>
    </footer>
  {/if}
</div>
