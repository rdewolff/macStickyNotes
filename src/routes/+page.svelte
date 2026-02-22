<script lang="ts">
  import { onMount } from "svelte";
  import StickyNote from "$lib/StickyNote.svelte";
  import NotesManager from "$lib/NotesManager.svelte";
  import { invoke } from "@tauri-apps/api/core";

  let isManager = false;
  if (typeof window !== "undefined") {
    // @ts-expect-error - set by tauri initialization script for manager window
    isManager = Boolean(window.__STICKY_MANAGER__);
  }

  async function applyThemeStylesheet() {
    try {
      const css = await invoke<string>("load_theme_stylesheet");
      let styleTag = document.getElementById("custom-theme-stylesheet") as HTMLStyleElement | null;
      if (!styleTag) {
        styleTag = document.createElement("style");
        styleTag.id = "custom-theme-stylesheet";
        document.head.appendChild(styleTag);
      }
      styleTag.textContent = css;
      window.dispatchEvent(new Event("sticky-theme-updated"));
    } catch (e) {
      console.error("Failed to load theme stylesheet:", e);
    }
  }

  onMount(() => {
    void applyThemeStylesheet();
  });
</script>

{#if isManager}
  <NotesManager />
{:else}
  <StickyNote />
{/if}
