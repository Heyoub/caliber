<script lang="ts">
  /**
   * Editor Layout
   * Shell for editor modes (assistant, playground)
   * Provides sidebar navigation and main content area
   */
  import { EditorLayout } from '$layouts';
  import { modeStore, type EditorMode } from '$stores/mode';

  interface Props {
    children: import('svelte').Snippet;
  }

  let { children }: Props = $props();

  // Get current mode from store
  let currentMode = $derived($modeStore);
</script>

<EditorLayout mode={currentMode}>
  {@render children()}
</EditorLayout>
