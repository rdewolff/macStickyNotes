import { v as pop, t as push, y as ensure_array_like, z as attr_class, F as attr, G as attr_style } from "../../chunks/index.js";
import "clsx";
import "quill";
import "quilljs-markdown";
import { webviewWindow } from "@tauri-apps/api";
import "@tauri-apps/api/dpi";
import "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
function Editor($$payload, $$props) {
  push();
  webviewWindow.getCurrentWebviewWindow();
  $$payload.out.push(`<div id="editor" class="svelte-1ad3fyg"></div>`);
  pop();
}
function _page($$payload, $$props) {
  push();
  const colors = [
    "#fff9b1",
    "#81B7DD",
    "#65A65B",
    "#AAD2CA",
    "#98C260",
    "#E1A1B1",
    "#B98CB3"
  ];
  const appWindow = webviewWindow.getCurrentWebviewWindow();
  let titlebarHovered = false;
  appWindow.listen("tauri://focus", (p) => {
    invoke("bring_all_to_front");
    document.body.classList.add("focused");
    appWindow.setAlwaysOnTop(true);
  });
  appWindow.listen("tauri://blur", () => {
    titlebarHovered = false;
    document.body.classList.remove("focused");
    appWindow.setAlwaysOnTop(false);
  });
  const each_array = ensure_array_like(colors);
  $$payload.out.push(`<div data-tauri-drag-region=""${attr_class("svelte-anft0o", void 0, { "hover": titlebarHovered })}><button class="titlebar-button svelte-anft0o" id="titlebar-close"><img src="https://api.iconify.design/mdi:close.svg" alt="close" class="svelte-anft0o"/></button> <button class="titlebar-button svelte-anft0o" id="titlebar-color"><img src="https://api.iconify.design/mdi:color.svg" alt="color" class="svelte-anft0o"/></button> <!--[-->`);
  for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
    let color = each_array[$$index];
    $$payload.out.push(`<button class="color svelte-anft0o"${attr("aria-label", color)}${attr_style("", {
      background: color,
      visibility: "hidden"
    })}></button>`);
  }
  $$payload.out.push(`<!--]--></div> `);
  Editor($$payload);
  $$payload.out.push(`<!---->`);
  pop();
}
export {
  _page as default
};
