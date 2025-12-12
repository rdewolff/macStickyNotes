use anyhow::Context;
use tauri::{Manager};
use std::sync::Mutex;

use crate::{
    save_load::{Note, save_sticky}, settings::MenuSettings, windows::close_sticky
};

#[tauri::command]
pub fn bring_all_to_front(window: tauri::Window, state: tauri::State<Mutex<MenuSettings>>) -> Result<(), String> {
    let settings = state.lock().map_err(|_| "Could not get lock on menu settings")?;
    if !settings.bring_to_front().map_err(|e| e.to_string())? {
        return Ok(())
    }

    window.webview_windows().iter().for_each(|(_, w)| {
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
        .context(format!(
            "Could not get position of window: {}",
            window.label()
        ))
        .map_err(|e| e.to_string())?
        .to_logical(scale_factor);

    let size = window
        .outer_size()
        .context(format!("Could not get size of window: {}", window.label()))
        .map_err(|e| e.to_string())?
        .to_logical(scale_factor);

    let note = Note {
        color,
        contents,
        x: position.x,
        y: position.y,
        height: size.height,
        width: size.width,
    };

    save_sticky(window.app_handle(), window.label(), Some(note)).map_err(|e| e.to_string())?;

    Ok(())
}
