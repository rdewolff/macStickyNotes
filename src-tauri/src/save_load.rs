use anyhow::Context;
use tauri_plugin_log::log;
use tauri_plugin_store::StoreExt;

use tauri::AppHandle;

use crate::windows::create_sticky;

const SAVE_PATH: &str = "save_data";

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Note {
    pub color: String,
    pub contents: String,
    pub x: i32,
    pub y: i32,
    pub height: u32,
    pub width: u32,
}

pub fn load_stickies(app: &AppHandle) -> Result<(), anyhow::Error> {
    let store = app.store(SAVE_PATH)?;

    if let Some(val) = store.get("data") {
        let map = val
            .as_object()
            .context("json key 'data' contained a non-object")?;

        let notes_vec: Vec<Note> = map
            .values()
            .map(|v| serde_json::from_value::<Note>(v.clone()))
            .collect::<Result<_, _>>()?;

        log::info!("loading stickies: {:?}", notes_vec);

        notes_vec.iter().for_each(|note| {
            if let Err(e) = create_sticky(app, Some(note)) {
                log::error!("Error creating window with payload: {:#}", e)
            }
        });
    };

    store.clear();
    Ok(())
}

// if data is None, window data is removed from store
pub fn save_sticky(app: &AppHandle, label: &str, note: Option<Note>) -> Result<(), anyhow::Error> {
    log::info!("Saving sticky: {:?}", note);

    let store = app.store(SAVE_PATH)?;

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
