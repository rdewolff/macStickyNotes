use std::iter::once;

use anyhow::Context;
use tauri::{Manager};

use crate::{
    save_load::{Note, save_sticky}, settings::MenuSettings, windows::{close_sticky, set_always_on_top, sorted_windows}
};

#[tauri::command]
pub fn bring_all_to_front(app: tauri::AppHandle, window: tauri::WebviewWindow, settings: tauri::State<MenuSettings>) -> Result<(), String> {
    if !settings.bring_to_front().map_err(|e| e.to_string())? {
        return Ok(())
    }

    sorted_windows(&app).into_iter().chain(once(window)).for_each(|w| {
        #[cfg(target_os = "macos")]
        {
            use objc2_app_kit::NSWindow;

            let ns_window_ptr = w.ns_window().unwrap();
            unsafe {
                let ns_window = &mut *(ns_window_ptr as *mut NSWindow);
                ns_window.orderFrontRegardless();
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn close_window(app: tauri::AppHandle) -> Result<(), String> {
    close_sticky(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_contents(
    window: tauri::WebviewWindow,
    color: String,
    contents: String,
) -> Result<(), String> {
    let scale_factor = window.scale_factor().map_err(|e| e.to_string())?;

    let position = window
        .outer_position()
        .map_err(|e| format!(
            "Could not get position of window: {} : {}",
            window.label(),
            e
        ))?
        .to_logical(scale_factor);

    let size = window
        .outer_size()
        .context(format!("Could not get size of window: {}", window.label()))
        .map_err(|e| e.to_string())?
        .to_logical(scale_factor);

    let always_on_top = window
        .is_always_on_top()
        .map_err(|e| format!(
            "Could not get window always_on_top status: {} : {}",
            window.label(),
            e
        ))?;

    let note = Note {
        color,
        contents,
        x: position.x,
        y: position.y,
        height: size.height,
        width: size.width,
        always_on_top
    };

    save_sticky(window.app_handle(), window.label(), Some(note)).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn set_note_always_on_top(app: tauri::AppHandle, always_on_top: bool) -> Result<(), String> {
    set_always_on_top(&app, always_on_top).map_err(|e| e.to_string())
}