use std::{fs, path::PathBuf};

use anyhow::{Context};
use chrono::{Duration, Local};
use tauri_plugin_log::log;
use tauri_plugin_store::StoreExt;

use tauri::{AppHandle, Manager};

use crate::{settings::MenuSettings, windows::create_sticky};

const NOTES_DATA: &str = "save_data";
const BACKUP_FOLDER: &str = "backups";
const SETTINGS: &str = "settings";

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Note {
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub contents: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub width: u32,
    #[serde(default)] // bool default is false
    pub always_on_top: bool,
    #[serde(default = "default_zoom")]
    pub zoom: f64,
}

fn default_zoom() -> f64 {
    1.0
}

pub fn create_backup(app: &AppHandle) -> anyhow::Result<()> {
    let app_data_dir = app.path().app_data_dir()
        .context("Failed to get app data directory")?;
    
    let store_path = app_data_dir.join(NOTES_DATA);
    
    if !store_path.exists() {
        return Ok(());
    }
    
    let backup_dir = app_data_dir.join(BACKUP_FOLDER);
    fs::create_dir_all(&backup_dir)
        .context("Failed to create backup directory")?;
    
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let backup_filename = format!("{}_{}", timestamp, NOTES_DATA);
    let backup_path = backup_dir.join(backup_filename);
    
    fs::copy(&store_path, &backup_path)
        .context("Failed to create backup")?;
    
    log::info!("Created backup: {:?}", backup_path);
    
    cleanup_old_backups(&backup_dir)?;
    
    Ok(())
}

// Remove backups older than 30 days
fn cleanup_old_backups(backup_dir: &PathBuf) -> anyhow::Result<()> {
    let cutoff_date = Local::now() - Duration::days(30);
    
    let entries = fs::read_dir(backup_dir)
        .context("Failed to read backup directory")?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if !path.is_file() {
            continue;
        }
        
        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                let modified_date = chrono::DateTime::<Local>::from(modified);
                
                if modified_date < cutoff_date {
                    match fs::remove_file(&path) {
                        Ok(_) => log::info!("Deleted old backup: {:?}", path),
                        Err(e) => log::warn!("Failed to delete backup {:?}: {}", path, e),
                    }
                }
            }
        }
    }
    
    Ok(())
}

pub fn load_stickies(app: &AppHandle) -> Result<(), anyhow::Error> {
    create_backup(app)?;
    
    let store = app.store(NOTES_DATA)?;

    if let Some(val) = store.get("data") {
        let map = val
            .as_object()
            .context("json key 'data' contained a non-object")?;

        let notes_vec: Vec<Note> = map
            .values()
            .map(|v| serde_json::from_value::<Note>(v.clone()))
            .collect::<Result<_, _>>()?;

        log::info!("loading stickies: {:#?}", notes_vec);

        let mut updated_map = serde_json::Map::new();

        notes_vec.into_iter().for_each(|note| match create_sticky(app, Some(&note)) {
            Ok(window) => {
                updated_map.insert(window.label().to_string(), serde_json::to_value(note).unwrap());
            },
            Err(e) => log::error!("Error creating window with payload: {:#}", e)
        });

        store.set("data", updated_map);
    } else {
        store.clear();
    }

    store.save()?;
    Ok(())
}

// if data is None, window data is removed from store
pub fn save_sticky(app: &AppHandle, label: &str, note: Option<Note>) -> Result<(), anyhow::Error> {
    log::info!("Saving sticky: {:?}", note);

    let store = app.store(NOTES_DATA)?;

    let mut value = store
        .get("data")
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    let data = value
        .as_object_mut()
        .context("json key 'data' contained a non-object")?;

    if let Some(note_data) = note {
        data.insert(label.to_string(), serde_json::to_value(note_data).unwrap());
    } else {
        log::debug!("deleting {} data from saved data", label);
        data.remove(&label.to_string());
    }

    store.set("data", value);
    store.save()?;

    Ok(())
}

pub fn load_settings(app: &AppHandle) -> anyhow::Result<MenuSettings> {
    log::info!("Loading settings");

    let store = app.store(SETTINGS)?;

    let bring_to_front = store.get("bring_to_front").and_then(|v| v.as_bool()).unwrap_or(true);
    let autostart = store.get("autostart").and_then(|v| v.as_bool()).unwrap_or(true);

    MenuSettings::new(app, bring_to_front, autostart)
}

pub fn save_settings(app: &AppHandle) -> anyhow::Result<()> {
    log::info!("Saving settings");

    let store = app.store(SETTINGS)?;
    let settings = app.state::<MenuSettings>();

    store.set("bring_to_front", settings.bring_to_front()?);

    Ok(())
}