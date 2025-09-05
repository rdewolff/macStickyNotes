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

    onMount(async () => {
        const quill = new Quill("#editor", {
            theme: "bubble",
            placeholder: "Empty Note",
            modules: {
                toolbar: false
            }
        });

        let timeout: undefined | number = $state()
        function debounceChangeEvent() {
            clearTimeout(timeout)
            timeout = setTimeout(() => 
                invoke("save_contents", {
                    contents: JSON.stringify(quill.getContents()),
                    color: document.body.style.backgroundColor,
                }), 
            2000)
        }

        quill.on("text-change", async () => {
            debounceChangeEvent()

            let editor = document.querySelector(".ql-editor");

            const factor = await appWindow.scaleFactor();

            const windowSize = (await appWindow.innerSize()).toLogical(factor);

            if (editor!.clientHeight > windowSize.height) {
                appWindow.setSize(
                    // 25 to get rid of the scroll bar
                    new LogicalSize(
                        windowSize.width,
                        editor!.clientHeight,
                    ),
                );
            }
        });

        new QuillMarkdown(quill, {});
        
        appWindow.listen("tauri://focus", () => quill.focus())

        requestAnimationFrame(() => quill.focus())

        appWindow.listen("fit_text", async () => {
          let editor = document.querySelector(".ql-editor");
          let maxWidth = 0;

          for (let item of editor!.children) {
            maxWidth = Math.max(maxWidth, item.clientWidth);
          }

          appWindow.setSize(
            new LogicalSize(maxWidth + 35, editor!.clientHeight)
          );
        });

        listen("save_request", () => 
            invoke("save_contents", {
                contents: JSON.stringify(quill.getContents()),
                color: document.body.style.backgroundColor,
            })
        )

        // @ts-expect-error
        let init = window.__STICKY_INIT__ as undefined | {contents: string, color: string}
        if (init) {
            quill.setContents(JSON.parse(init.contents));
            document.body.style.backgroundColor = init.color;
        } else {
            document.body.style.backgroundColor = "#fff9b1";
        }

        document.addEventListener("click", () => {
            quill.focus()
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
