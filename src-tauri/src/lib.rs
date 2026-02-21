use std::time::Duration;

use tauri::{App, Emitter, Manager};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_log::log::{self, LevelFilter};
use tauri_plugin_updater::UpdaterExt;

use crate::commands::*;
use crate::menu::{create_menu, handle_menu_event};
use crate::save_load::{load_settings, load_stickies};

mod anchor;
mod commands;
mod menu;
mod save_load;
mod settings;
mod windows;

fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    load_stickies(app.handle())?;

    let menu_settings = load_settings(app.handle())?;

    let autostart_manager = app.autolaunch();
    if !cfg!(debug_assertions) {
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            update(handle).await.unwrap();
        });

        if !autostart_manager.is_enabled()? {
            if menu_settings.autostart()? {
                autostart_manager.enable()?;
            } else {
                autostart_manager.disable()?;
            }
        }
    } else {
        autostart_manager.disable()?;
    }
    log::info!(
        "registered for autostart? {}",
        autostart_manager.is_enabled()?
    );

    app.manage(menu_settings);
    app.manage(anchor::AnchorState::default());

    let menu = create_menu(app.handle())?;
    app.set_menu(menu)?;
    app.on_menu_event(handle_menu_event);

    Ok(())
}

async fn update(app: tauri::AppHandle) -> tauri_plugin_updater::Result<()> {
    if let Some(update) = app.updater()?.check().await? {
        let mut downloaded = 0;

        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    log::info!("downloaded {downloaded} from {content_length:?}");
                },
                || {
                    log::info!("download finished");
                },
            )
            .await?;

        log::info!("update installed");
        app.restart();
    }

    Ok(())
}

pub fn run() {
    let mut allow_exit_after_flush = false;

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(LevelFilter::Debug)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            bring_all_to_front,
            save_contents,
            close_window,
            set_note_always_on_top,
            anchor_to_nearest,
            unanchor,
            open_note_manager_window,
            list_saved_notes,
            restore_note,
            archive_note,
            delete_note,
            open_notes_folder,
        ])
        .setup(setup)
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(move |app, event| match event {
            // prevent app from exiting when no windows are open
            tauri::RunEvent::ExitRequested { api, code, .. } => {
                if allow_exit_after_flush {
                    return;
                }

                if code.is_none() {
                    api.prevent_exit();
                } else {
                    log::info!("exit code: {:?}", code);
                    api.prevent_exit();

                    allow_exit_after_flush = true;
                    let app_handle = app.clone();
                    let exit_code = code.unwrap_or_default();

                    let _ = app_handle.emit("save_request", ());
                    tauri::async_runtime::spawn_blocking(move || {
                        std::thread::sleep(Duration::from_millis(220));
                        app_handle.exit(exit_code);
                    });
                }
            }
            _ => {}
        });
}
