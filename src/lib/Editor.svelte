<script lang="ts">
  import { onMount } from "svelte";
  import Quill from "quill";
  // @ts-expect-error
  import QuillMarkdown from "quilljs-markdown";
  import "quill/dist/quill.bubble.css";
  import { webviewWindow } from "@tauri-apps/api";
  import { LogicalSize } from "@tauri-apps/api/dpi";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";

  const appWindow = webviewWindow.getCurrentWebviewWindow();
  const RESIZE_THRESHOLD_PX = 2;

  type StickyInit = {
    id: string;
    contents: string;
    color: string;
  };

  type DeltaOp = {
    insert?: string | Record<string, unknown>;
  };

  type QuillDelta = {
    ops?: DeltaOp[];
  };

  let quill: undefined | Quill = $state();
  let saveTimeout: null | number = null;
  let noteId = $state("");
  let applyingExternalUpdate = false;

  function defaultNoteColor(): string {
    const style = getComputedStyle(document.documentElement);
    return (
      style.getPropertyValue("--sticky-default-color").trim() ||
      style.getPropertyValue("--sticky-color-1").trim() ||
      "#f9e7a7"
    );
  }

  function getNoteColor(): string {
    const container = document.getElementById("note-container");
    return container?.style.backgroundColor || defaultNoteColor();
  }

  function getZoomLevel(): number {
    const container = document.getElementById("note-container");
    const zoom = container?.style.zoom;
    return zoom ? parseFloat(zoom) : 1.0;
  }

  function parseDelta(contents: string): QuillDelta | null {
    try {
      return JSON.parse(contents) as QuillDelta;
    } catch {
      return null;
    }
  }

  function isMeaningfulSerializedContents(contents: string): boolean {
    const trimmed = contents.trim();
    if (trimmed.length === 0) {
      return false;
    }

    const delta = parseDelta(contents);
    if (!delta || !Array.isArray(delta.ops)) {
      return true;
    }

    if (delta.ops.length === 0) {
      return false;
    }

    for (const op of delta.ops) {
      const insert = op?.insert;
      if (typeof insert === "string") {
        if (insert.replace(/\n/g, "").trim().length > 0) {
          return true;
        }
        continue;
      }

      if (insert && typeof insert === "object") {
        return true;
      }
    }

    return false;
  }

  function serializeContents(): string {
    if (!quill) {
      return "";
    }

    const delta = quill.getContents() as QuillDelta;
    const hasEmbed =
      delta.ops?.some(
        (op) => op.insert !== null && typeof op.insert === "object",
      ) ?? false;

    if (!hasEmbed && quill.getText().trim().length === 0) {
      return "";
    }

    return JSON.stringify(delta);
  }

  async function persistContents() {
    if (!quill || !noteId) {
      return;
    }

    await invoke("save_contents", {
      noteId,
      contents: serializeContents(),
      color: getNoteColor(),
      zoom: getZoomLevel(),
    });
  }

  export async function save_contents(force = false) {
    if (saveTimeout) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }

    if (force) {
      await persistContents();
      return;
    }

    saveTimeout = setTimeout(() => {
      void persistContents();
      saveTimeout = null;
    }, 300);
  }

  export function remove_selection() {
    quill?.setSelection(null);
  }

  export function apply_external_contents(contents: string) {
    if (!quill) {
      return;
    }

    applyingExternalUpdate = true;
    if (isMeaningfulSerializedContents(contents)) {
      const delta = parseDelta(contents);
      if (delta) {
        quill.setContents(delta as any);
      } else {
        quill.setText(contents);
      }
    } else {
      quill.setText("");
    }
    applyingExternalUpdate = false;
  }

  async function growWindowToFitEditorContent(editor: HTMLElement) {
    const overflowHeight = Math.ceil(editor.scrollHeight - editor.clientHeight);
    if (overflowHeight <= RESIZE_THRESHOLD_PX) {
      return;
    }

    const factor = await appWindow.scaleFactor();
    const windowSize = (await appWindow.innerSize()).toLogical(factor);

    await appWindow.setSize(
      new LogicalSize(windowSize.width, windowSize.height + overflowHeight),
    );
  }

  onMount(async () => {
    quill = new Quill("#editor", {
      theme: "bubble",
      placeholder: "",
      modules: {
        toolbar: false,
      },
    });

    quill.keyboard.addBinding({ key: "a", shortKey: true }, () => {
      if (!quill) {
        return false;
      }

      const contentLength = Math.max(quill.getLength() - 1, 0);
      quill.setSelection(0, contentLength, "user");
      return false;
    });

    // @ts-expect-error - set by tauri initialization script for sticky windows
    const init = window.__STICKY_INIT__ as StickyInit | undefined;

    const noteContainer = document.getElementById("note-container");
    if (init?.id) {
      noteId = init.id;
    }

    if (init?.contents && isMeaningfulSerializedContents(init.contents)) {
      const delta = parseDelta(init.contents);
      if (delta) {
        quill.setContents(delta as any);
      } else {
        quill.setText(init.contents);
      }
    }

    if (noteContainer) {
      noteContainer.style.backgroundColor = init?.color || defaultNoteColor();
    }

    quill.on("text-change", async (_delta, _oldDelta, source) => {
      if (!applyingExternalUpdate && source !== "silent") {
        void save_contents();
      }

      const editor = document.querySelector(".ql-editor");
      if (editor instanceof HTMLElement) {
        await growWindowToFitEditorContent(editor);
      }
    });

    // remove color and background color formatting
    quill.clipboard.matchers.splice(5, 1);

    new QuillMarkdown(quill, {});

    requestAnimationFrame(() => quill?.focus());

    appWindow.listen("fit_text", async () => {
      const editor = document.querySelector(".ql-editor") as HTMLElement;

      editor.style.minHeight = "fit-content";

      const factor = await appWindow.scaleFactor();
      const windowSize = (await appWindow.outerSize()).toLogical(factor);

      requestAnimationFrame(async () => {
        appWindow.setSize(new LogicalSize(windowSize.width, editor.clientHeight));
        editor.style.minHeight = "calc(100vh - 8px)";
      });
    });

    listen("save_request", () => {
      void save_contents(true);
    });

    const flushOnExit = () => {
      void save_contents(true);
    };

    window.addEventListener("beforeunload", flushOnExit);
    window.addEventListener("pagehide", flushOnExit);
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "hidden") {
        void save_contents(true);
      }
    });
  });
</script>

<div id="editor"></div>

<style>
  #editor {
    width: 100%;
    height: 100%;
  }
</style>
