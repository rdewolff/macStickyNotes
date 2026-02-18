<script lang="ts">
  import StickyNote from "$lib/StickyNote.svelte";
  import NotesManager from "$lib/NotesManager.svelte";

  let isManager = false;
  if (typeof window !== "undefined") {
    // @ts-expect-error - set by tauri initialization script for manager window
    isManager = Boolean(window.__STICKY_MANAGER__);
  }
</script>

{#if isManager}
  <NotesManager />
{:else}
  <StickyNote />
{/if}
