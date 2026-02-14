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

    let quill: undefined | Quill = $state();
    let saveTimeout: null | number = null;

    function getNoteColor(): string {
        const container = document.getElementById("note-container");
        return container?.style.backgroundColor || "#fff9b1";
    }

    function getZoomLevel(): number {
        const container = document.getElementById("note-container");
        const zoom = container?.style.zoom;
        return zoom ? parseFloat(zoom) : 1.0;
    }

    export async function save_contents() {
        if (saveTimeout) {
            clearTimeout(saveTimeout)
        }
        saveTimeout = setTimeout(async () => {
            if (quill) {
                await invoke("save_contents", {
                    contents: JSON.stringify(quill.getContents()),
                    color: getNoteColor(),
                    zoom: getZoomLevel(),
                });
            }
        }, 300);
    }

    export function remove_selection() {
        quill?.setSelection(null)
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
            placeholder: "Empty Note",
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

        // @ts-expect-error
        let init = window.__STICKY_INIT__ as
            | undefined
            | { contents: string; color: string };
        const noteContainer = document.getElementById("note-container");
        if (init) {
            quill.setContents(JSON.parse(init.contents));
            if (noteContainer) noteContainer.style.backgroundColor = init.color;
        } else {
            if (noteContainer) noteContainer.style.backgroundColor = "#fff9b1";
        }

        let timeout: undefined | number = $state();
        function debounceChangeEvent() {
            clearTimeout(timeout);
            timeout = setTimeout(save_contents, 2000);
        }

        quill.on("text-change", async () => {
            debounceChangeEvent();

            const editor = document.querySelector(".ql-editor");
            if (editor instanceof HTMLElement) {
                await growWindowToFitEditorContent(editor);
            }
        });

        // remove color and background color formatting
        quill.clipboard.matchers.splice(5, 1)

        new QuillMarkdown(quill, {});

        requestAnimationFrame(() => quill?.focus());

        appWindow.listen("fit_text", async () => {
            let editor = document.querySelector(".ql-editor") as HTMLElement;

            editor.style.minHeight = "fit-content";

            const factor = await appWindow.scaleFactor();
            const windowSize = (await appWindow.outerSize()).toLogical(factor);

            requestAnimationFrame(async () => {
                appWindow.setSize(
                    new LogicalSize(windowSize.width, editor!.clientHeight),
                );
                editor.style.minHeight = "calc(100vh - 8px)";
            });
        });

        listen("save_request", save_contents);
    });
</script>

<div id="editor"></div>

<style>
    #editor {
        width: 100%;
        height: 100%;
    }
</style>
