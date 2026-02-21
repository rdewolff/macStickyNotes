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

  const colors = [
    "#fff9b1",
    "#81B7DD",
    "#65A65B",
    "#AAD2CA",
    "#98C260",
    "#E1A1B1",
    "#B98CB3",
  ];
  const appWindow = webviewWindow.getCurrentWebviewWindow();

  let editor: Editor;

  let colorMenuOpen = $state(false);
  let titlebarHovered = $state(false);
  let alwaysOnTop = $state(false);
  let anchored = $state(false);
  let anchorTarget = $state("");

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
    // @ts-expect-error - set by tauri initialization script for sticky windows
    if (!window.__STICKY_INIT__) {
      document.body.classList.add("focused");
      return;
    }

    // @ts-expect-error - set by tauri initialization script for sticky windows
    alwaysOnTop = Boolean(window.__STICKY_INIT__?.always_on_top);
    // @ts-expect-error - set by tauri initialization script for sticky windows
    const initZoom = window.__STICKY_INIT__?.zoom;
    if (initZoom && initZoom !== 1.0) {
      const container = document.getElementById("note-container");
      if (container) container.style.zoom = String(initZoom);
    }
  });
</script>

<div class="note-container" id="note-container">
  <div data-tauri-drag-region class="titlebar" class:hover={titlebarHovered}>
    <button class="titlebar-button" id="titlebar-close" onclick={closeNote} aria-label="close note">
      <svg-icon class="cross" type="mdi" path={mdiClose} size="15"></svg-icon>
    </button>
    <button class="titlebar-button" id="titlebar-pin" onclick={toggleAlwaysOnTop} aria-label="pin/unpin note">
      <svg-icon class="cross" type="mdi" path={alwaysOnTop ? mdiPinOff : mdiPin} size="10"></svg-icon>
    </button>
    <button class="titlebar-button" id="titlebar-anchor" onclick={toggleAnchor} aria-label="anchor to window" class:anchored={anchored}>
      <svg-icon class="cross" type="mdi" path={anchored ? mdiLinkOff : mdiLink} size="10"></svg-icon>
    </button>
    <button class="titlebar-button" id="titlebar-manager" onclick={openManager} aria-label="manage notes">
      <svg-icon class="cross" type="mdi" path={mdiFormatListBulleted} size="12"></svg-icon>
    </button>
    <button class="titlebar-button" id="titlebar-color" onclick={toggleColorMenu} aria-label="select note color">
      <svg-icon class="cross" type="mdi" path={mdiPalette} size="10"></svg-icon>
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
    height: 24px;
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
    transition: opacity 0.2s ease, background-color 0.2s ease;
    border-radius: 12px 12px 0 0;
  }

  .titlebar.hover {
    opacity: 1;
    background: rgba(0, 0, 0, 0.08);
  }

  button {
    border: 0;
    margin: 0;
    padding: 0;
    height: 24px;
    width: 24px;
    background-color: transparent;
    border-radius: 4px;
    transition: background-color 0.15s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  button:hover {
    background-color: rgba(0, 0, 0, 0.1);
  }

  .color {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    margin: 4px 2px;
  }

  .anchored {
    color: rgba(0, 0, 0, 0.7);
  }

  .anchor-badge {
    font-size: 10px;
    line-height: 24px;
    padding: 0 6px;
    opacity: 0.6;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100px;
    user-select: none;
  }
</style>
