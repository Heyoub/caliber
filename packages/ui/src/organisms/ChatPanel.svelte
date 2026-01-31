<!--
  ChatPanel.svelte - Full chat interface organism
  Ported from ChatPanel.vue

  Features:
  - Message list with virtual scrolling support
  - Input area with attachments
  - Tool call display area
  - Drag and drop file upload
  - View mode switching
  - Svelte 5 runes
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { ChatMessage, ToolCall, CMSContent } from '../types/index.js';

  type ViewMode = 'chat' | 'prompts' | 'templates' | 'reasoning' | 'history' | 'documents';

  interface Props {
    /** Content from CMS */
    cms?: CMSContent;
    /** Chat messages */
    messages?: ChatMessage[];
    /** Current tool calls */
    toolCalls?: ToolCall[];
    /** Currently sending message */
    sending?: boolean;
    /** Current view mode */
    viewMode?: ViewMode;
    /** Enable file drag-drop */
    enableDragDrop?: boolean;
    /** Additional CSS classes */
    class?: string;
    /** Header slot */
    header?: Snippet;
    /** Message list slot */
    messageList?: Snippet;
    /** Input area slot */
    inputArea?: Snippet;
    /** Tool calls display slot */
    toolCallsArea?: Snippet;
    /** Empty state slot */
    emptyState?: Snippet;
    /** Event handlers */
    onSend?: (message: string) => void;
    onFileUpload?: (files: File[]) => void;
    onViewChange?: (view: ViewMode) => void;
  }

  let {
    cms = {},
    messages = [],
    toolCalls = [],
    sending = false,
    viewMode = 'chat',
    enableDragDrop = true,
    class: className = '',
    header,
    messageList,
    inputArea,
    toolCallsArea,
    emptyState,
    onSend,
    onFileUpload,
    onViewChange
  }: Props = $props();

  // State
  let dragActive = $state(false);
  let dragCounter = $state(0);
  let messageContainerRef: HTMLDivElement | undefined = $state();

  // Derived
  const isEmpty = $derived(messages.length === 0);
  const hasToolCalls = $derived(toolCalls.length > 0);

  // Drag-and-drop handlers
  function handleDragEnter(event: DragEvent) {
    if (!enableDragDrop) return;
    if (event.dataTransfer?.types.includes('Files')) {
      dragCounter++;
      dragActive = true;
    }
  }

  function handleDragLeave() {
    if (!enableDragDrop) return;
    dragCounter--;
    if (dragCounter <= 0) {
      dragActive = false;
      dragCounter = 0;
    }
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragCounter = 0;
    dragActive = false;

    const fileList = event.dataTransfer?.files;
    if (fileList && fileList.length > 0) {
      const files = Array.from(fileList);
      onFileUpload?.(files);
    }
  }

  // Scroll to bottom when new messages arrive
  $effect(() => {
    if (messages.length && messageContainerRef) {
      messageContainerRef.scrollTop = messageContainerRef.scrollHeight;
    }
  });

  // View mode tabs
  const viewTabs: { id: ViewMode; label: string }[] = [
    { id: 'chat', label: cms.chatLabel || 'Chat' },
    { id: 'prompts', label: cms.promptsLabel || 'Prompts' },
    { id: 'templates', label: cms.templatesLabel || 'Templates' },
    { id: 'history', label: cms.historyLabel || 'History' }
  ];
</script>

<div
  class={`relative flex flex-col h-full min-h-[400px] bg-slate-900/0 rounded-2xl ${className}`}
  ondragenter={handleDragEnter}
  ondragleave={handleDragLeave}
  ondragover={handleDragOver}
  ondrop={handleDrop}
>
  <!-- Gradient Background Layer -->
  <div class="absolute inset-0 bg-gradient-to-br from-purple-500/20 via-teal-500/15 to-coral-500/20 blur-2xl opacity-60 rounded-2xl -z-10"></div>

  <!-- Header -->
  {#if header}
    <div class="relative z-10 border-b border-slate-700/50">
      {@render header()}
    </div>
  {:else}
    <header class="relative z-10 flex items-center justify-between px-4 py-3 border-b border-slate-700/50 backdrop-blur-sm">
      <div class="flex items-center gap-2">
        <div class="w-2 h-2 rounded-full bg-mint-400 animate-pulse"></div>
        <span class="text-sm font-medium text-slate-200">
          {cms.title || 'Chat'}
        </span>
        {#if sending}
          <span class="text-xs text-slate-400">{cms.sendingLabel || 'Sending...'}</span>
        {/if}
      </div>

      <!-- View mode tabs -->
      <nav class="flex gap-1">
        {#each viewTabs as tab}
          <button
            class={`px-3 py-1.5 text-xs font-medium rounded-lg transition-colors ${
              viewMode === tab.id
                ? 'bg-teal-500/20 text-teal-300'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-700/50'
            }`}
            onclick={() => onViewChange?.(tab.id)}
          >
            {tab.label}
          </button>
        {/each}
      </nav>
    </header>
  {/if}

  <!-- Drag-and-drop overlay -->
  {#if dragActive}
    <div class="absolute inset-0 flex flex-col items-center justify-center bg-slate-900/80 border-2 border-dashed border-teal-400 rounded-2xl pointer-events-none z-30">
      <svg class="w-12 h-12 text-teal-300 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
      </svg>
      <span class="text-teal-100 font-medium text-lg">{cms.dropLabel || 'Drop files to upload'}</span>
    </div>
  {/if}

  <!-- Main Content Area -->
  <div
    bind:this={messageContainerRef}
    class="flex-1 overflow-y-auto relative z-10 p-4"
  >
    {#if viewMode === 'chat'}
      {#if isEmpty && emptyState}
        <div class="flex items-center justify-center h-full">
          {@render emptyState()}
        </div>
      {:else if messageList}
        {@render messageList()}
      {:else}
        <!-- Default message list rendering -->
        <div class="space-y-4">
          {#each messages as message (message.id)}
            <div class="animate-in slide-in-from-bottom-2">
              <!-- Message would be rendered here using ChatMessage component -->
              <div class="text-slate-300 text-sm">{message.content}</div>
            </div>
          {/each}
        </div>
      {/if}

      <!-- Tool calls area -->
      {#if hasToolCalls}
        <div class="mt-4 pt-4 border-t border-slate-700/50">
          {#if toolCallsArea}
            {@render toolCallsArea()}
          {:else}
            <div class="text-xs text-slate-400 mb-2">{cms.toolCallsLabel || 'Tool Calls'}</div>
            <div class="space-y-2">
              {#each toolCalls as call (call.id)}
                <div class="text-sm text-slate-300 bg-slate-800/50 rounded-lg p-2">
                  {call.name}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    {:else}
      <!-- Other view modes would render here -->
      <div class="flex items-center justify-center h-full text-slate-400">
        {cms.viewPlaceholder || `${viewMode} view`}
      </div>
    {/if}
  </div>

  <!-- Input Area -->
  {#if viewMode === 'chat'}
    <div class="relative z-10 border-t border-slate-700/50 p-4 backdrop-blur-sm">
      {#if inputArea}
        {@render inputArea()}
      {:else}
        <div class="flex items-end gap-2">
          <div class="flex-1 relative">
            <textarea
              class="w-full px-4 py-3 bg-slate-800/50 backdrop-blur-sm text-slate-100 rounded-xl border border-slate-600/50 resize-none focus:outline-none focus:ring-2 focus:ring-teal-500/50 focus:border-teal-500/50 placeholder-slate-500"
              placeholder={cms.inputPlaceholder || 'Type a message...'}
              rows="1"
              disabled={sending}
            ></textarea>
          </div>
          <button
            class="px-4 py-3 bg-teal-500 hover:bg-teal-400 text-white rounded-xl font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={sending}
          >
            {#if sending}
              <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
            {:else}
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
              </svg>
            {/if}
          </button>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Loading overlay -->
  {#if sending}
    <div class="absolute inset-0 flex items-center justify-center bg-slate-900/30 backdrop-blur-sm z-20 pointer-events-none">
      <div class="flex items-center gap-2 text-slate-200">
        <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        <span class="font-medium">{cms.processingLabel || 'Processing...'}</span>
      </div>
    </div>
  {/if}
</div>

<style>
  /* Smooth scroll for message container */
  .overflow-y-auto {
    scroll-behavior: smooth;
  }

  /* Animation for new messages */
  @keyframes slide-in-from-bottom {
    from {
      opacity: 0;
      transform: translateY(0.5rem);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .animate-in {
    animation: slide-in-from-bottom 0.2s ease-out;
  }
</style>
