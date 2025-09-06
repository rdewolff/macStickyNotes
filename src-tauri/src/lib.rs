use tauri::App;
use tauri_plugin_log::log::{self, LevelFilter};
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};


use crate::commands::*;
use crate::menu::{create_menu, handle_menu_event};
use crate::save_load::load_stickies;

mod commands;
mod menu;
mod save_load;
mod windows;

fn setup(app: &mut App) -> Result<(), Box<(dyn std::error::Error)>> {
    load_stickies(app.handle())?;

    let menu = create_menu(app.handle())?;
    app.set_menu(menu)?;
    app.on_menu_event(handle_menu_event);

    let autostart_manager = app.autolaunch();
    if !cfg!(debug_assertions) {

        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            update(handle).await.unwrap();
        });

        if !autostart_manager.is_enabled()? {
            autostart_manager.enable()?;
        }
    } else {
        autostart_manager.disable()?;
    }
    log::info!("registered for autostart? {}", autostart_manager.is_enabled()?);

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
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
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
            close_window
        ])
        .setup(setup)
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app, event| match event {
            tauri::RunEvent::ExitRequested { api, code, .. } => {
                if code.is_none() {
                    api.prevent_exit();
                } else {
                    log::info!("exit code: {:?}", code);
                }
            }
            _ => {}
        });
}
