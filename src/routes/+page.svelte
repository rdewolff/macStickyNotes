<script lang="ts">
  import Editor from "$lib/Editor.svelte";
  import { onMount } from "svelte";
  import { webviewWindow } from "@tauri-apps/api";
  import { invoke } from "@tauri-apps/api/core";
  import { mdiClose, mdiPalette, mdiPin, mdiPinOff } from '@mdi/js';
  import "@jamescoyle/svg-icon"

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
  let alwaysOnTop = $state(false)

  async function toggleAlwaysOnTop() {
    alwaysOnTop = !alwaysOnTop
    await invoke("set_note_always_on_top", {alwaysOnTop})
    await editor.save_contents()
  }

  function closeNote() {
    invoke("close_window")
  }

  function toggleColorMenu() {
    colorMenuOpen = !colorMenuOpen;
  }

  function handleColorClick(e: MouseEvent) {
    document.body.style.backgroundColor = (
      e.target as HTMLDivElement
    ).style.backgroundColor;
    editor.save_contents();
    toggleColorMenu();
  }

  appWindow.listen("tauri://focus", async (p) => {
    await invoke("bring_all_to_front")
    document.body.classList.add("focused")
  })
  
  appWindow.listen("tauri://blur", () => {
    titlebarHovered = false
    document.body.classList.remove("focused")
    editor?.remove_selection()
  })

  appWindow.listen<number>("set_color", (event) => {
    document.body.style.backgroundColor = colors[event.payload];
  })

  let move_timer: number | undefined = undefined;
  function save_debounce() {
    clearTimeout(move_timer)
    setTimeout(async () => await editor?.save_contents(), 100)
  }

  appWindow.listen("tauri://move", save_debounce)
  appWindow.listen("tauri://resize", save_debounce)

  onMount(async () => {
    // @ts-expect-error
    if (!window.__STICKY_INIT__) {
      document.body.classList.add("focused") // window is focused on creation by default, except when initialized
    } else {
      //@ts-expect-error
      alwaysOnTop = window.__STICKY_INIT__.always_on_top
    }
    
    document.body.addEventListener("mouseenter", () => titlebarHovered = true);
    document.body.addEventListener("mouseleave", () => titlebarHovered = false);
  });
</script>

<div data-tauri-drag-region class:hover={titlebarHovered}>
  <button class="titlebar-button" id="titlebar-close" onclick={closeNote} aria-label="close note">
    <svg-icon class="cross" type="mdi" path={mdiClose} size="15"></svg-icon>
  </button>
  <button class="titlebar-button" id="titlebar-close" onclick={toggleAlwaysOnTop} aria-label="pin/unpin note">
    <svg-icon class="cross" type="mdi" path={alwaysOnTop ? mdiPinOff : mdiPin} size="10"></svg-icon>
  </button>
  <button class="titlebar-button" id="titlebar-color" onclick={toggleColorMenu} aria-label="select note color">
     <svg-icon class="cross" type="mdi" path={mdiPalette} size="10"></svg-icon>
  </button>
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

<style>
  div {
    height: 20px;
    user-select: none;
    display: flex;
    justify-content: flex-start;
    flex-direction: row-reverse;
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 1;
    opacity: 0.1;
    transition: all cubic-bezier(0.165, 0.84, 0.44, 1) 0.25s;
  }

  .hover {
    opacity: 1 !important;
    background: rgba(0, 0, 0, 0.1);
  }

  button {
    border: 0;
    margin: 0;
    padding: 0;
    height: 20px;
    width: 20px;
    background-color: transparent;
  }
</style>
