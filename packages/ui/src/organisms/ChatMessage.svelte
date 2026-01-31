<!--
  ChatMessage.svelte - Single chat message organism
  Ported from ChatBubble.vue

  Features:
  - User vs assistant styling
  - Markdown rendering slot
  - Streaming indicator
  - Copy to clipboard
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { ChatMessageData, ColorPalette, CMSContent } from '../types/index.js';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Message data */
    message: ChatMessageData;
    /** Show typing indicator */
    streaming?: boolean;
    /** Enable markdown rendering */
    renderMarkdown?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Avatar slot */
    avatar?: Snippet;
    /** Content slot for custom rendering */
    content?: Snippet;
    /** Actions slot (copy, etc) */
    actions?: Snippet;
  }

  let {
    cms = {},
    message,
    streaming = false,
    renderMarkdown = true,
    class: className = '',
    avatar,
    content,
    actions
  }: Props = $props();

  // State
  let isCopying = $state(false);

  // Derived values
  const isUser = $derived(message.role === 'user');
  const isAssistant = $derived(message.role === 'assistant');
  const isSystem = $derived(message.role === 'system');
  const isStreaming = $derived(streaming || message.typing);

  // Role colors
  const roleColors: Record<string, { text: string; bg: string; border: string }> = {
    user: {
      text: 'text-coral-400',
      bg: 'bg-coral-500/10',
      border: 'border-coral-500/30'
    },
    assistant: {
      text: 'text-purple-400',
      bg: 'bg-purple-500/10',
      border: 'border-purple-500/30'
    },
    system: {
      text: 'text-slate-400',
      bg: 'bg-slate-500/10',
      border: 'border-slate-500/30'
    }
  };

  const colors = $derived(roleColors[message.role] || roleColors.assistant);

  // Container classes
  const containerClasses = $derived(
    `flex items-start gap-3 p-3 ${isUser ? 'flex-row-reverse' : 'flex-row'} ${className}`
  );

  // Bubble classes
  const bubbleClasses = $derived(
    `relative flex-1 min-w-0 max-w-full p-4 rounded-xl shadow-md
     bg-slate-700/20 backdrop-blur-lg border border-solid border-slate-500/30
     text-slate-100`
  );

  // Copy to clipboard
  async function handleCopy() {
    if (isCopying) return;

    try {
      isCopying = true;
      await navigator.clipboard.writeText(message.content);
      setTimeout(() => {
        isCopying = false;
      }, 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
      isCopying = false;
    }
  }

  // Format timestamp
  const formattedTime = $derived(
    message.timestamp
      ? new Intl.DateTimeFormat('en', {
          hour: 'numeric',
          minute: '2-digit'
        }).format(message.timestamp)
      : ''
  );

  // Escape HTML for user messages
  function escapeHtml(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }
</script>

<!-- System message (centered, italic) -->
{#if isSystem}
  <div class="text-center text-sm italic text-slate-400 my-2 px-4">
    {message.content}
  </div>
{:else}
  <!-- User or Assistant message bubble -->
  <div class={containerClasses}>
    <!-- Avatar -->
    <div class={`flex-shrink-0 ${isUser ? 'order-last ml-2' : 'mr-2'}`}>
      {#if avatar}
        {@render avatar()}
      {:else}
        <div class="w-8 h-8 rounded-full flex items-center justify-center bg-slate-800/80 shadow-sm border border-slate-700/50">
          {#if isUser}
            <svg class="w-4 h-4 text-coral-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
            </svg>
          {:else}
            <svg class="w-4 h-4 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
            </svg>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Message Content Area -->
    <div class={bubbleClasses}>
      <!-- Role Label -->
      <div class={`text-xs font-medium mb-1.5 ${colors.text}`}>
        {isUser ? (cms.userLabel || 'User') : (message.model || cms.assistantLabel || 'Assistant')}
      </div>

      <!-- Model metadata -->
      {#if isAssistant && message.model}
        <div class="text-[0.625rem] text-slate-500 opacity-70 mb-1">
          {message.model}
        </div>
      {/if}

      <!-- Typing/Streaming indicator -->
      {#if isStreaming}
        <div class="flex items-center gap-3 p-1">
          <div class="w-6 h-6 flex-shrink-0 rounded-full bg-gradient-to-br from-purple-500/30 to-coral-500/20 flex items-center justify-center">
            <svg class="w-3 h-3 text-purple-400" fill="currentColor" viewBox="0 0 24 24">
              <path d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.456 2.456L21.75 6l-1.035.259a3.375 3.375 0 00-2.456 2.456zM16.894 20.567L16.5 21.75l-.394-1.183a2.25 2.25 0 00-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 001.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 001.423 1.423l1.183.394-1.183.394a2.25 2.25 0 00-1.423 1.423z" />
            </svg>
          </div>
          <div class="dot-flashing"></div>
        </div>
      {:else}
        <!-- Message content -->
        {#if content}
          {@render content()}
        {:else}
          <div class="text-sm break-words whitespace-pre-wrap">
            {#if isUser}
              {message.content}
            {:else}
              <!-- For assistant, render as HTML (markdown should be pre-processed) -->
              {@html renderMarkdown ? message.content : escapeHtml(message.content)}
            {/if}
          </div>
        {/if}
      {/if}

      <!-- Timestamp -->
      {#if formattedTime}
        <div class="text-[0.625rem] text-slate-500 mt-2">
          {formattedTime}
        </div>
      {/if}

      <!-- Copy button -->
      <button
        onclick={handleCopy}
        title={isCopying ? (cms.copyingLabel || 'Copying...') : (cms.copyLabel || 'Copy to clipboard')}
        class="absolute top-2 right-2 p-1 rounded hover:bg-slate-700/60 transition opacity-0 group-hover:opacity-100"
        class:opacity-100={isCopying}
      >
        {#if isCopying}
          <svg class="w-4 h-4 text-mint-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
        {:else}
          <svg class="w-4 h-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
          </svg>
        {/if}
      </button>

      <!-- Custom actions slot -->
      {#if actions}
        <div class="mt-2 pt-2 border-t border-slate-700/50">
          {@render actions()}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* Typing indicator animation */
  .dot-flashing {
    position: relative;
    width: 6px;
    height: 6px;
    border-radius: 5px;
    background-color: hsl(var(--purple-400));
    animation: dot-flashing 1s infinite linear alternate;
    animation-delay: 0.5s;
  }

  .dot-flashing::before,
  .dot-flashing::after {
    content: '';
    display: inline-block;
    position: absolute;
    top: 0;
    width: 6px;
    height: 6px;
    border-radius: 5px;
    background-color: hsl(var(--purple-400));
  }

  .dot-flashing::before {
    left: -10px;
    animation: dot-flashing 1s infinite alternate;
    animation-delay: 0s;
  }

  .dot-flashing::after {
    left: 10px;
    animation: dot-flashing 1s infinite alternate;
    animation-delay: 1s;
  }

  @keyframes dot-flashing {
    0% { background-color: hsl(var(--purple-400)); }
    50%, 100% { background-color: hsl(var(--purple-400) / 0.2); }
  }

  /* Group hover for copy button */
  .group:hover .group-hover\:opacity-100 {
    opacity: 1;
  }
</style>
