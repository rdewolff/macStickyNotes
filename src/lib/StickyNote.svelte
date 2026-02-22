<script lang="ts">
  import Editor from "$lib/Editor.svelte";
  import { onMount } from "svelte";
  import { webviewWindow } from "@tauri-apps/api";
  import { invoke } from "@tauri-apps/api/core";
  import {
    mdiClose,
    mdiFormatListBulleted,
    mdiLink,
    mdiLinkOff,
    mdiPalette,
    mdiPin,
    mdiPinOff,
  } from "@mdi/js";
  import "@jamescoyle/svg-icon";

  const fallbackColors = [
    "#f9e7a7",
    "#bddcf6",
    "#bfe6bf",
    "#cfeee8",
    "#d6e8b6",
    "#edc2ce",
    "#d7c2e9",
  ];
  const appWindow = webviewWindow.getCurrentWebviewWindow();

  let editor: Editor;
  let colors = $state<string[]>([...fallbackColors]);

  let colorMenuOpen = $state(false);
  let titlebarHovered = $state(false);
  let alwaysOnTop = $state(false);
  let anchored = $state(false);
  let anchorTarget = $state("");

  type ExternalNoteUpdatePayload = {
    contents: string;
    color: string;
    zoom: number;
  };

  async function toggleAlwaysOnTop() {
    alwaysOnTop = !alwaysOnTop;
    await invoke("set_note_always_on_top", { alwaysOnTop });
    await editor.save_contents(true);
  }

  async function toggleAnchor() {
    if (anchored) {
      await invoke("unanchor");
      anchored = false;
      anchorTarget = "";
    } else {
      try {
        const targetName = await invoke<string>("anchor_to_nearest");
        anchored = true;
        anchorTarget = targetName;
      } catch (e) {
        console.error("Anchor failed:", e);
      }
    }
  }

  async function closeNote() {
    await editor.save_contents(true);
    await invoke("close_window");
  }

  async function openManager() {
    await invoke("open_note_manager_window");
  }

  function toggleColorMenu() {
    colorMenuOpen = !colorMenuOpen;
  }

  function setNoteColor(color: string) {
    const container = document.getElementById("note-container");
    if (container) container.style.backgroundColor = color;
  }

  function refreshPaletteFromTheme() {
    if (typeof window === "undefined") {
      return;
    }

    const style = getComputedStyle(document.documentElement);
    colors = fallbackColors.map((fallback, index) => {
      const configured = style.getPropertyValue(`--sticky-color-${index + 1}`).trim();
      return configured || fallback;
    });
  }

  function handleColorClick(e: MouseEvent) {
    setNoteColor((e.target as HTMLButtonElement).style.backgroundColor);
    editor.save_contents(true);
    toggleColorMenu();
  }

  appWindow.listen("tauri://focus", async () => {
    await invoke("bring_all_to_front");
    titlebarHovered = true;
    document.body.classList.add("focused");
  });

  appWindow.listen("tauri://blur", async () => {
    titlebarHovered = false;
    document.body.classList.remove("focused");
    editor?.remove_selection();
    await editor?.save_contents(true);
  });

  appWindow.listen<number>("set_color", (event) => {
    setNoteColor(colors[event.payload]);
  });

  appWindow.listen("anchor_lost", () => {
    anchored = false;
    anchorTarget = "";
  });

  appWindow.listen<string>("anchor_set", (event) => {
    anchored = true;
    anchorTarget = event.payload;
  });

  appWindow.listen<string>("zoom", (event) => {
    const container = document.getElementById("note-container");
    if (!container) return;
    let current = parseFloat(container.style.zoom || "1");
    if (event.payload === "in") {
      current = Math.min(current + 0.1, 2.0);
    } else if (event.payload === "out") {
      current = Math.max(current - 0.1, 0.5);
    } else if (event.payload === "reset") {
      current = 1.0;
    }
    container.style.zoom = String(current);
    editor?.save_contents(true);
  });

  appWindow.listen<ExternalNoteUpdatePayload>("external_note_update", (event) => {
    const container = document.getElementById("note-container");
    if (container) {
      if (event.payload.color) {
        container.style.backgroundColor = event.payload.color;
      }
      if (event.payload.zoom && Number.isFinite(event.payload.zoom)) {
        container.style.zoom = String(event.payload.zoom);
      }
    }

    editor?.apply_external_contents(event.payload.contents);
  });

  let moveTimer: number | undefined = undefined;
  function saveDebounce() {
    if (moveTimer) {
      clearTimeout(moveTimer);
    }
    moveTimer = setTimeout(async () => {
      await editor?.save_contents();
    }, 100);
  }

  appWindow.listen("tauri://move", saveDebounce);
  appWindow.listen("tauri://resize", saveDebounce);

  onMount(() => {
    refreshPaletteFromTheme();
    const themeListener = () => refreshPaletteFromTheme();
    window.addEventListener("sticky-theme-updated", themeListener);

    // @ts-expect-error - set by tauri initialization script for sticky windows
    if (!window.__STICKY_INIT__) {
      document.body.classList.add("focused");
      return () => {
        window.removeEventListener("sticky-theme-updated", themeListener);
      };
    }

    // @ts-expect-error - set by tauri initialization script for sticky windows
    alwaysOnTop = Boolean(window.__STICKY_INIT__?.always_on_top);
    // @ts-expect-error - set by tauri initialization script for sticky windows
    const initZoom = window.__STICKY_INIT__?.zoom;
    if (initZoom && initZoom !== 1.0) {
      const container = document.getElementById("note-container");
      if (container) container.style.zoom = String(initZoom);
    }

    return () => {
      window.removeEventListener("sticky-theme-updated", themeListener);
    };
  });
</script>

<div class="note-container" id="note-container">
  <div data-tauri-drag-region class="titlebar" class:hover={titlebarHovered}>
    <button class="titlebar-button" id="titlebar-close" onclick={closeNote} aria-label="close note">
      <svg-icon class="cross" type="mdi" path={mdiClose} size="18"></svg-icon>
    </button>
    <button class="titlebar-button" id="titlebar-pin" onclick={toggleAlwaysOnTop} aria-label="pin/unpin note">
      <svg-icon class="cross" type="mdi" path={alwaysOnTop ? mdiPinOff : mdiPin} size="14"></svg-icon>
    </button>
    <button class="titlebar-button" id="titlebar-anchor" onclick={toggleAnchor} aria-label="anchor to window" class:anchored={anchored}>
      <svg-icon class="cross" type="mdi" path={anchored ? mdiLinkOff : mdiLink} size="14"></svg-icon>
    </button>
    <button class="titlebar-button" id="titlebar-manager" onclick={openManager} aria-label="manage notes">
      <svg-icon class="cross" type="mdi" path={mdiFormatListBulleted} size="15"></svg-icon>
    </button>
    <button class="titlebar-button" id="titlebar-color" onclick={toggleColorMenu} aria-label="select note color">
      <svg-icon class="cross" type="mdi" path={mdiPalette} size="14"></svg-icon>
    </button>
    {#if anchored}
      <span class="anchor-badge">{anchorTarget}</span>
    {/if}
    {#each colors as color}
      <button
        class="color"
        onclick={handleColorClick}
        aria-label={color}
        style:background={color}
        style:visibility={colorMenuOpen ? "visible" : "hidden"}
      ></button>
    {/each}
  </div>

  <Editor bind:this={editor} />
</div>

<style>
  .titlebar {
    height: var(--sticky-titlebar-height, 30px);
    user-select: none;
    display: flex;
    justify-content: flex-start;
    flex-direction: row-reverse;
    position: fixed;
    top: 4px;
    left: 4px;
    right: 4px;
    z-index: 3;
    opacity: 0;
    transition: opacity 0.2s ease;
    border-radius:
      var(--sticky-corner-top-left, var(--sticky-corner-radius, 12px))
      var(--sticky-corner-top-right, var(--sticky-corner-radius, 12px))
      0
      0;
  }

  .titlebar.hover {
    opacity: 1;
  }

  button {
    border: 0;
    margin: 0;
    padding: 0;
    height: var(--sticky-titlebar-button-size, 28px);
    width: var(--sticky-titlebar-button-size, 28px);
    background-color: transparent;
    border-radius: 6px;
    transition: background-color 0.15s ease, transform 0.15s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  button:hover {
    background-color: rgba(0, 0, 0, 0.1);
    transform: scale(1.08);
  }

  .color {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    margin: 5px 2px;
  }

  .anchored {
    color: rgba(0, 0, 0, 0.7);
  }

  .anchor-badge {
    font-size: 10px;
    line-height: var(--sticky-titlebar-height, 30px);
    padding: 0 6px;
    opacity: 0.6;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100px;
    user-select: none;
  }
</style>
