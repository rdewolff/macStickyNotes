<script lang="ts">
  import Editor from "$lib/Editor.svelte";
  import { onMount } from "svelte";
  import { webviewWindow } from "@tauri-apps/api";
  import { invoke } from "@tauri-apps/api/core";

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

  appWindow.listen("tauri://focus", (p) => {
    invoke("bring_all_to_front")
    document.body.classList.add("focused")
    appWindow.setAlwaysOnTop(true);
  })
  
  appWindow.listen("tauri://blur", () => {
    titlebarHovered = false
    document.body.classList.remove("focused")
    editor.remove_selection()
    appWindow.setAlwaysOnTop(false);
  })

  appWindow.listen<number>("set_color", (event) => {
    document.body.style.backgroundColor = colors[event.payload];
  })

  onMount(async () => {
    // @ts-expect-error
    if (!window.__STICKY_INIT__) {
      document.body.classList.add("focused") // window is focused on creation by default, except when initialized
    }

    document.body.addEventListener("mouseenter", () => titlebarHovered = true);
    document.body.addEventListener("mouseleave", () => titlebarHovered = false);
  });
</script>

<div data-tauri-drag-region class:hover={titlebarHovered}>
  <button class="titlebar-button" id="titlebar-close" onclick={closeNote}>
    <img src="https://api.iconify.design/mdi:close.svg" alt="close" />
  </button>
  <button class="titlebar-button" id="titlebar-color" onclick={toggleColorMenu}>
    <img src="https://api.iconify.design/mdi:color.svg" alt="color" />
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
  }

  img {
    height: 10px;
    padding: 5px;
  }
</style>
