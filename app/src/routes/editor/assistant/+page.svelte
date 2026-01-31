<script lang="ts">
  /**
   * Assistant Mode Page
   * Real API-backed editor for AI agent interactions
   */
  import { Card, Button, Icon, Badge, Spinner } from '@caliber/ui';
  import { assistantStore } from '$stores/assistant';
  import { editorStore } from '$stores/editor';
  import { memoryStore } from '$stores/memory';
  import type { PageData } from './$types';

  interface Props {
    data: PageData;
  }

  let { data }: Props = $props();

  // Content strings
  const content = {
    title: 'Assistant Mode',
    subtitle: 'AI-powered memory editing with real API',
    panels: {
      chat: 'Chat',
      editor: 'Editor',
      memory: 'Memory Graph',
    },
    actions: {
      send: 'Send',
      clear: 'Clear',
      save: 'Save',
    },
    placeholders: {
      message: 'Ask the assistant to help with memory operations...',
    },
    empty: {
      chat: 'Start a conversation with the assistant',
      editor: 'Select a file to edit',
      memory: 'No memory graph loaded',
    },
  };

  // Local state
  let messageInput = $state('');
  let sending = $state(false);

  // Derived state from stores
  let messages = $derived($assistantStore.messages);
  let activeFile = $derived($editorStore.activeFile);
  let trajectory = $derived($memoryStore.trajectory);

  async function sendMessage() {
    if (!messageInput.trim() || sending) return;

    const message = messageInput.trim();
    messageInput = '';
    sending = true;

    try {
      await assistantStore.sendMessage(message);
    } catch (error) {
      console.error('Failed to send message:', error);
    } finally {
      sending = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      sendMessage();
    }
  }
</script>

<svelte:head>
  <title>Assistant - CALIBER Editor</title>
</svelte:head>

<div class="assistant-page">
  <!-- Left Panel: Chat -->
  <section class="panel chat-panel">
    <header class="panel-header">
      <h2 class="panel-title">
        <Icon name="message-square" size="sm" color="purple" />
        {content.panels.chat}
      </h2>
      <Badge color="purple" glow="subtle">Assistant</Badge>
    </header>

    <div class="chat-messages">
      {#if messages.length === 0}
        <div class="empty-state">
          <Icon name="message-circle" size="xl" color="slate" />
          <p>{content.empty.chat}</p>
        </div>
      {:else}
        {#each messages as message}
          <div class="message" class:assistant={message.role === 'assistant'} class:user={message.role === 'user'}>
            <div class="message-avatar">
              <Icon name={message.role === 'assistant' ? 'bot' : 'user'} size="sm" />
            </div>
            <div class="message-content">
              <p>{message.content}</p>
              {#if message.toolCalls && message.toolCalls.length > 0}
                <div class="tool-calls">
                  {#each message.toolCalls as toolCall}
                    <Badge color={toolCall.status === 'success' ? 'mint' : toolCall.status === 'error' ? 'coral' : 'amber'} size="sm">
                      <Icon name="tool" size="xs" />
                      {toolCall.name}
                    </Badge>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {/each}
      {/if}
    </div>

    <div class="chat-input">
      <textarea
        bind:value={messageInput}
        onkeydown={handleKeydown}
        placeholder={content.placeholders.message}
        rows="3"
        disabled={sending}
      ></textarea>
      <Button
        color="purple"
        glow={messageInput.trim().length > 0 ? 'subtle' : undefined}
        loading={sending}
        disabled={!messageInput.trim()}
        onclick={sendMessage}
      >
        <Icon name="send" size="sm" />
        {content.actions.send}
      </Button>
    </div>
  </section>

  <!-- Center Panel: Editor -->
  <section class="panel editor-panel">
    <header class="panel-header">
      <h2 class="panel-title">
        <Icon name="code" size="sm" color="teal" />
        {content.panels.editor}
      </h2>
      {#if activeFile}
        <Badge color="teal" size="sm">{activeFile.name}</Badge>
      {/if}
    </header>

    <div class="editor-content">
      {#if activeFile}
        <!-- File tabs would go here -->
        <div class="editor-tabs">
          {#each $editorStore.openFiles as file}
            <button
              class="editor-tab"
              class:active={file.path === activeFile.path}
              onclick={() => editorStore.setActiveFile(file)}
            >
              <Icon name="file" size="xs" />
              {file.name}
            </button>
          {/each}
        </div>
        <!-- Editor component would render here -->
        <div class="editor-area">
          <pre><code>{activeFile.content || '// Loading...'}</code></pre>
        </div>
      {:else}
        <div class="empty-state">
          <Icon name="file-text" size="xl" color="slate" />
          <p>{content.empty.editor}</p>
        </div>
      {/if}
    </div>
  </section>

  <!-- Right Panel: Memory Graph -->
  <section class="panel memory-panel">
    <header class="panel-header">
      <h2 class="panel-title">
        <Icon name="git-branch" size="sm" color="pink" />
        {content.panels.memory}
      </h2>
    </header>

    <div class="memory-content">
      {#if trajectory}
        <div class="trajectory-info">
          <h3>{trajectory.name}</h3>
          <div class="scope-list">
            {#each trajectory.scopes as scope}
              <div class="scope-item" class:active={scope.id === $memoryStore.activeScope?.id}>
                <Badge color="teal" size="sm">{scope.name}</Badge>
                <span class="event-count">{scope.eventCount} events</span>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="empty-state">
          <Icon name="git-branch" size="xl" color="slate" />
          <p>{content.empty.memory}</p>
        </div>
      {/if}
    </div>
  </section>
</div>

<style>
  .assistant-page {
    display: grid;
    grid-template-columns: 1fr 1.5fr 1fr;
    gap: var(--space-4);
    height: 100%;
    min-height: 0;
  }

  .panel {
    display: flex;
    flex-direction: column;
    background: hsl(var(--slate-900) / 0.5);
    border: 1px solid hsl(var(--slate-700));
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid hsl(var(--slate-700));
    background: hsl(var(--slate-800) / 0.5);
  }

  .panel-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-family: var(--font-display);
    font-size: var(--text-sm);
    font-weight: 600;
    color: hsl(var(--text-primary));
    margin: 0;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    color: hsl(var(--text-muted));
    font-size: var(--text-sm);
  }

  /* Chat Panel */
  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .message {
    display: flex;
    gap: var(--space-3);
    padding: var(--space-3);
    border-radius: var(--radius-md);
  }

  .message.user {
    background: hsl(var(--slate-800) / 0.5);
  }

  .message.assistant {
    background: hsl(var(--purple-500) / 0.1);
    border: 1px solid hsl(var(--purple-500) / 0.2);
  }

  .message-avatar {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: hsl(var(--slate-700));
    border-radius: var(--radius-full);
    flex-shrink: 0;
  }

  .message-content {
    flex: 1;
  }

  .message-content p {
    margin: 0;
    font-size: var(--text-sm);
    color: hsl(var(--text-primary));
    white-space: pre-wrap;
  }

  .tool-calls {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .chat-input {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border-top: 1px solid hsl(var(--slate-700));
    background: hsl(var(--slate-800) / 0.3);
  }

  .chat-input textarea {
    width: 100%;
    padding: var(--space-3);
    background: hsl(var(--slate-800));
    border: 1px solid hsl(var(--slate-700));
    border-radius: var(--radius-md);
    color: hsl(var(--text-primary));
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    resize: none;
  }

  .chat-input textarea:focus {
    outline: none;
    border-color: hsl(var(--purple-500) / 0.5);
  }

  .chat-input textarea::placeholder {
    color: hsl(var(--text-muted));
  }

  /* Editor Panel */
  .editor-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .editor-tabs {
    display: flex;
    gap: var(--space-1);
    padding: var(--space-2);
    background: hsl(var(--slate-800) / 0.5);
    border-bottom: 1px solid hsl(var(--slate-700));
    overflow-x: auto;
  }

  .editor-tab {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: hsl(var(--text-muted));
    font-size: var(--text-xs);
    cursor: pointer;
    white-space: nowrap;
  }

  .editor-tab:hover {
    background: hsl(var(--slate-700) / 0.5);
    color: hsl(var(--text-secondary));
  }

  .editor-tab.active {
    background: hsl(var(--teal-500) / 0.15);
    color: hsl(var(--teal-400));
  }

  .editor-area {
    flex: 1;
    overflow: auto;
    padding: var(--space-4);
  }

  .editor-area pre {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .editor-area code {
    color: hsl(var(--text-primary));
  }

  /* Memory Panel */
  .memory-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
  }

  .trajectory-info h3 {
    font-family: var(--font-display);
    font-size: var(--text-base);
    font-weight: 600;
    color: hsl(var(--text-primary));
    margin: 0 0 var(--space-4) 0;
  }

  .scope-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .scope-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3);
    background: hsl(var(--slate-800) / 0.5);
    border: 1px solid hsl(var(--slate-700));
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .scope-item:hover {
    background: hsl(var(--slate-700) / 0.5);
  }

  .scope-item.active {
    border-color: hsl(var(--teal-500) / 0.5);
    background: hsl(var(--teal-500) / 0.1);
  }

  .event-count {
    font-size: var(--text-xs);
    color: hsl(var(--text-muted));
  }

  /* Responsive */
  @media (max-width: 1024px) {
    .assistant-page {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr auto;
    }
  }
</style>
