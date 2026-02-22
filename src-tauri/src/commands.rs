use std::{iter::once, path::PathBuf, process::Command};

use anyhow::Context;
use tauri::{Emitter, Manager};

use crate::{
    anchor,
    save_load::{
        delete_note as delete_note_record, get_notes_directory_path, list_notes,
        load_theme_stylesheet as load_theme_stylesheet_content, mark_note_archived, mark_note_open,
        notes_directory, restart_notes_directory_watcher, save_sticky, set_notes_directory_path,
        Note, NoteListItem, NoteStatus,
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
    let status = list_notes(&app)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|record| record.id == note_id)
        .map(|record| record.status)
        .ok_or_else(|| format!("Note not found: {note_id}"))?;

    if status != NoteStatus::Archived {
        return Err("Only archived notes can be deleted permanently".to_string());
    }

    let confirmed = matches!(
        rfd::MessageDialog::new()
            .set_title("Delete archived note?")
            .set_description("Are you sure you want to permanently delete this archived note?")
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show(),
        rfd::MessageDialogResult::Yes
    );

    if !confirmed {
        return Ok(());
    }

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

#[tauri::command]
pub fn get_notes_folder(app: tauri::AppHandle) -> Result<String, String> {
    get_notes_directory_path(&app)
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_notes_folder(app: tauri::AppHandle, folder_path: String) -> Result<String, String> {
    let cleaned = folder_path.trim();
    if cleaned.is_empty() {
        return Err("Folder path cannot be empty".to_string());
    }

    let configured = set_notes_directory_path(&app, PathBuf::from(cleaned))
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();
    restart_notes_directory_watcher(&app).map_err(|e| e.to_string())?;
    let _ = app.emit("notes_changed", ());
    Ok(configured)
}

#[tauri::command]
pub fn choose_notes_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let current_dir = get_notes_directory_path(&app).ok();
    let mut dialog = rfd::FileDialog::new();
    if let Some(path) = current_dir {
        dialog = dialog.set_directory(path);
    }

    let Some(path) = dialog.pick_folder() else {
        return Ok(None);
    };

    let configured = set_notes_directory_path(&app, path).map_err(|e| e.to_string())?;
    restart_notes_directory_watcher(&app).map_err(|e| e.to_string())?;
    let _ = app.emit("notes_changed", ());

    Ok(Some(configured.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn load_theme_stylesheet(app: tauri::AppHandle) -> Result<String, String> {
    load_theme_stylesheet_content(&app).map_err(|e| e.to_string())
}
