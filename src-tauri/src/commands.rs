use std::{iter::once, process::Command};

use anyhow::Context;
use tauri::{Emitter, Manager};

use crate::{
    anchor,
    save_load::{
        delete_note as delete_note_record, list_notes, mark_note_archived, mark_note_open,
        notes_directory, save_sticky, Note, NoteListItem,
    },
    settings::MenuSettings,
    windows::{
        close_sticky, close_sticky_by_note_id, create_sticky, open_note_manager, set_always_on_top,
        sorted_windows,
    },
};

#[tauri::command]
pub fn bring_all_to_front(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    settings: tauri::State<MenuSettings>,
) -> Result<(), String> {
    if !settings.bring_to_front().map_err(|e| e.to_string())? {
        return Ok(());
    }

    sorted_windows(&app)
        .into_iter()
        .chain(once(window))
        .for_each(|w| {
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
    close_sticky(&app).map_err(|e| e.to_string())?;
    let _ = app.emit("notes_changed", ());
    Ok(())
}

#[tauri::command]
pub fn save_contents(
    window: tauri::WebviewWindow,
    note_id: String,
    color: String,
    contents: String,
    zoom: Option<f64>,
) -> Result<(), String> {
    let scale_factor = window.scale_factor().map_err(|e| e.to_string())?;

    let position = window
        .outer_position()
        .map_err(|e| {
            format!(
                "Could not get position of window: {} : {}",
                window.label(),
                e
            )
        })?
        .to_logical(scale_factor);

    let size = window
        .outer_size()
        .context(format!("Could not get size of window: {}", window.label()))
        .map_err(|e| e.to_string())?
        .to_logical(scale_factor);

    let always_on_top = window.is_always_on_top().map_err(|e| {
        format!(
            "Could not get window always_on_top status: {} : {}",
            window.label(),
            e
        )
    })?;

    let note = Note {
        color,
        contents,
        x: position.x,
        y: position.y,
        height: size.height,
        width: size.width,
        always_on_top,
        zoom: zoom.unwrap_or(1.0),
    };

    save_sticky(window.app_handle(), &note_id, note).map_err(|e| e.to_string())?;

    let _ = window.app_handle().emit("notes_changed", ());

    Ok(())
}

#[tauri::command]
pub fn set_note_always_on_top(app: tauri::AppHandle, always_on_top: bool) -> Result<(), String> {
    set_always_on_top(&app, always_on_top).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn anchor_to_nearest(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<String, String> {
    anchor::anchor_to_nearest(&app, &window).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unanchor(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    anchor::unanchor(&app, &window).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_note_manager_window(app: tauri::AppHandle) -> Result<(), String> {
    open_note_manager(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_saved_notes(app: tauri::AppHandle) -> Result<Vec<NoteListItem>, String> {
    list_notes(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_note(app: tauri::AppHandle, note_id: String) -> Result<(), String> {
    let record = mark_note_open(&app, &note_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Note not found: {note_id}"))?;

    create_sticky(&app, Some(&record)).map_err(|e| e.to_string())?;
    let _ = app.emit("notes_changed", ());
    Ok(())
}

#[tauri::command]
pub fn archive_note(app: tauri::AppHandle, note_id: String) -> Result<(), String> {
    close_sticky_by_note_id(&app, &note_id).map_err(|e| e.to_string())?;
    mark_note_archived(&app, &note_id).map_err(|e| e.to_string())?;
    let _ = app.emit("notes_changed", ());
    Ok(())
}

#[tauri::command]
pub fn delete_note(app: tauri::AppHandle, note_id: String) -> Result<(), String> {
    close_sticky_by_note_id(&app, &note_id).map_err(|e| e.to_string())?;
    delete_note_record(&app, &note_id).map_err(|e| e.to_string())?;
    let _ = app.emit("notes_changed", ());
    Ok(())
}

#[tauri::command]
pub fn open_notes_folder(app: tauri::AppHandle) -> Result<(), String> {
    let notes_dir = notes_directory(&app).map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(notes_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(notes_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(notes_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
