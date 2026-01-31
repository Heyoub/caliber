<script lang="ts">
  /**
   * Playground Mode Page
   * Client-side sandbox for local experimentation
   * Uses IndexedDB for persistence
   */
  import { Card, Button, Icon, Badge, Spinner } from '@caliber/ui';
  import { playgroundStore } from '$stores/playground';
  import { editorStore } from '$stores/editor';
  import type { PageData } from './$types';

  interface Props {
    data: PageData;
  }

  let { data }: Props = $props();

  // Content strings
  const content = {
    title: 'Playground Mode',
    subtitle: 'Local sandbox for experimentation',
    panels: {
      files: 'Files',
      editor: 'Editor',
      preview: 'Preview',
    },
    actions: {
      newFile: 'New File',
      import: 'Import',
      export: 'Export',
      save: 'Save',
      run: 'Run',
    },
    placeholders: {
      filename: 'filename.yaml',
    },
    empty: {
      files: 'No files yet. Create or import one.',
      editor: 'Select a file to edit',
      preview: 'Edit a file to see preview',
    },
    fileTypes: ['yaml', 'toml', 'json', 'md', 'csv'],
  };

  // Local state
  let newFileName = $state('');
  let newFileType = $state('yaml');
  let showNewFileForm = $state(false);
  let saving = $state(false);

  // Derived state from stores
  let files = $derived($playgroundStore.files);
  let activeFile = $derived($editorStore.activeFile);
  let editorContent = $state('');

  // Update editor content when active file changes
  $effect(() => {
    if (activeFile) {
      editorContent = activeFile.content || '';
    }
  });

  async function createNewFile() {
    if (!newFileName.trim()) return;

    const name = newFileName.includes('.')
      ? newFileName
      : `${newFileName}.${newFileType}`;

    await playgroundStore.createFile(name, '');
    newFileName = '';
    showNewFileForm = false;
  }

  async function saveCurrentFile() {
    if (!activeFile) return;

    saving = true;
    try {
      await playgroundStore.updateFile(activeFile.path, editorContent);
      editorStore.updateActiveFileContent(editorContent);
    } finally {
      saving = false;
    }
  }

  async function deleteFile(path: string) {
    if (confirm('Are you sure you want to delete this file?')) {
      await playgroundStore.deleteFile(path);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    // Ctrl/Cmd + S to save
    if ((event.ctrlKey || event.metaKey) && event.key === 's') {
      event.preventDefault();
      saveCurrentFile();
    }
  }

  function getFileIcon(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'yaml':
      case 'yml':
        return 'file-text';
      case 'toml':
        return 'file-code';
      case 'json':
        return 'file-json';
      case 'md':
        return 'file-text';
      case 'csv':
        return 'table';
      default:
        return 'file';
    }
  }
</script>

<svelte:head>
  <title>Playground - CALIBER Editor</title>
</svelte:head>

<svelte:window onkeydown={handleKeydown} />

<div class="playground-page">
  <!-- Left Panel: Files -->
  <section class="panel files-panel">
    <header class="panel-header">
      <h2 class="panel-title">
        <Icon name="folder" size="sm" color="amber" />
        {content.panels.files}
      </h2>
      <Button size="sm" color="ghost" onclick={() => (showNewFileForm = !showNewFileForm)}>
        <Icon name="plus" size="sm" />
      </Button>
    </header>

    {#if showNewFileForm}
      <div class="new-file-form">
        <input
          type="text"
          bind:value={newFileName}
          placeholder={content.placeholders.filename}
          class="new-file-input"
        />
        <select bind:value={newFileType} class="new-file-select">
          {#each content.fileTypes as type}
            <option value={type}>.{type}</option>
          {/each}
        </select>
        <Button size="sm" color="teal" onclick={createNewFile} disabled={!newFileName.trim()}>
          Create
        </Button>
      </div>
    {/if}

    <div class="files-list">
      {#if files.length === 0}
        <div class="empty-state">
          <Icon name="folder-open" size="xl" color="slate" />
          <p>{content.empty.files}</p>
        </div>
      {:else}
        {#each files as file}
          <button
            class="file-item"
            class:active={activeFile?.path === file.path}
            onclick={() => editorStore.openFile(file)}
          >
            <Icon name={getFileIcon(file.name)} size="sm" />
            <span class="file-name">{file.name}</span>
            <button
              class="file-delete"
              onclick={(e) => {
                e.stopPropagation();
                deleteFile(file.path);
              }}
            >
              <Icon name="x" size="xs" />
            </button>
          </button>
        {/each}
      {/if}
    </div>

    <div class="panel-footer">
      <Button size="sm" color="ghost" fullWidth>
        <Icon name="upload" size="sm" />
        {content.actions.import}
      </Button>
      <Button size="sm" color="ghost" fullWidth disabled={files.length === 0}>
        <Icon name="download" size="sm" />
        {content.actions.export}
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
        <div class="editor-actions">
          <Badge color="slate" size="sm">{activeFile.name.split('.').pop()}</Badge>
          <Button
            size="sm"
            color="teal"
            glow="subtle"
            loading={saving}
            onclick={saveCurrentFile}
          >
            <Icon name="save" size="sm" />
            {content.actions.save}
          </Button>
        </div>
      {/if}
    </header>

    <div class="editor-content">
      {#if activeFile}
        <textarea
          bind:value={editorContent}
          class="editor-textarea"
          spellcheck="false"
        ></textarea>
      {:else}
        <div class="empty-state">
          <Icon name="file-text" size="xl" color="slate" />
          <p>{content.empty.editor}</p>
        </div>
      {/if}
    </div>
  </section>

  <!-- Right Panel: Preview -->
  <section class="panel preview-panel">
    <header class="panel-header">
      <h2 class="panel-title">
        <Icon name="eye" size="sm" color="mint" />
        {content.panels.preview}
      </h2>
    </header>

    <div class="preview-content">
      {#if activeFile && editorContent}
        <pre class="preview-code"><code>{editorContent}</code></pre>
      {:else}
        <div class="empty-state">
          <Icon name="eye" size="xl" color="slate" />
          <p>{content.empty.preview}</p>
        </div>
      {/if}
    </div>
  </section>
</div>

<style>
  .playground-page {
    display: grid;
    grid-template-columns: 250px 1fr 300px;
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

  .panel-footer {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-3);
    border-top: 1px solid hsl(var(--slate-700));
    background: hsl(var(--slate-800) / 0.3);
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
    padding: var(--space-8);
  }

  /* New File Form */
  .new-file-form {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-3);
    border-bottom: 1px solid hsl(var(--slate-700));
    background: hsl(var(--slate-800) / 0.3);
  }

  .new-file-input {
    flex: 1;
    padding: var(--space-2);
    background: hsl(var(--slate-800));
    border: 1px solid hsl(var(--slate-700));
    border-radius: var(--radius-sm);
    color: hsl(var(--text-primary));
    font-size: var(--text-sm);
  }

  .new-file-input:focus {
    outline: none;
    border-color: hsl(var(--teal-500) / 0.5);
  }

  .new-file-select {
    padding: var(--space-2);
    background: hsl(var(--slate-800));
    border: 1px solid hsl(var(--slate-700));
    border-radius: var(--radius-sm);
    color: hsl(var(--text-primary));
    font-size: var(--text-sm);
  }

  /* Files List */
  .files-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2);
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: hsl(var(--text-secondary));
    font-size: var(--text-sm);
    cursor: pointer;
    text-align: left;
    transition: all var(--duration-fast) var(--ease-default);
  }

  .file-item:hover {
    background: hsl(var(--slate-700) / 0.5);
    color: hsl(var(--text-primary));
  }

  .file-item.active {
    background: hsl(var(--teal-500) / 0.15);
    color: hsl(var(--teal-400));
  }

  .file-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-delete {
    opacity: 0;
    padding: var(--space-1);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: hsl(var(--text-muted));
    cursor: pointer;
  }

  .file-item:hover .file-delete {
    opacity: 1;
  }

  .file-delete:hover {
    background: hsl(var(--coral-500) / 0.2);
    color: hsl(var(--coral-400));
  }

  /* Editor Panel */
  .editor-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .editor-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .editor-textarea {
    flex: 1;
    width: 100%;
    padding: var(--space-4);
    background: hsl(var(--slate-900));
    border: none;
    color: hsl(var(--text-primary));
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    line-height: 1.6;
    resize: none;
  }

  .editor-textarea:focus {
    outline: none;
  }

  /* Preview Panel */
  .preview-content {
    flex: 1;
    overflow: auto;
    padding: var(--space-4);
  }

  .preview-code {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .preview-code code {
    color: hsl(var(--text-primary));
  }

  /* Responsive */
  @media (max-width: 1024px) {
    .playground-page {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr auto;
    }

    .files-panel {
      max-height: 200px;
    }

    .preview-panel {
      max-height: 300px;
    }
  }
</style>
