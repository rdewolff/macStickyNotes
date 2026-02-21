use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use chrono::{Duration, Local, Utc};
use tauri_plugin_log::log;
use tauri_plugin_store::StoreExt;

use tauri::{AppHandle, Manager};

use crate::{settings::MenuSettings, windows::create_sticky};

const NOTES_DATA: &str = "save_data";
const BACKUP_FOLDER: &str = "backups";
const MARKDOWN_NOTES_FOLDER: &str = "notes";
const SETTINGS: &str = "settings";

static NOTE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Note {
    #[serde(default = "default_note_color")]
    pub color: String,
    #[serde(default)]
    pub contents: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default = "default_note_height")]
    pub height: u32,
    #[serde(default = "default_note_width")]
    pub width: u32,
    #[serde(default)] // bool default is false
    pub always_on_top: bool,
    #[serde(default = "default_zoom")]
    pub zoom: f64,
}

impl Default for Note {
    fn default() -> Self {
        Self {
            color: default_note_color(),
            contents: String::new(),
            x: 0,
            y: 0,
            height: default_note_height(),
            width: default_note_width(),
            always_on_top: false,
            zoom: default_zoom(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NoteStatus {
    Open,
    Closed,
    Archived,
}

impl Default for NoteStatus {
    fn default() -> Self {
        Self::Open
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct NoteRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub status: NoteStatus,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub closed_at: Option<String>,
    #[serde(default)]
    pub archived_at: Option<String>,
    #[serde(flatten)]
    pub note: Note,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct NoteListItem {
    pub id: String,
    pub status: NoteStatus,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub archived_at: Option<String>,
    pub color: String,
    pub preview: String,
}

fn default_zoom() -> f64 {
    1.0
}

fn default_note_color() -> String {
    "#fff9b1".to_string()
}

fn default_note_height() -> u32 {
    250
}

fn default_note_width() -> u32 {
    300
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn sanitize_note_id(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();

    if cleaned.is_empty() {
        generate_note_id()
    } else {
        cleaned
    }
}

fn ensure_unique_id(candidate: String, used: &mut HashSet<String>) -> String {
    if !used.contains(&candidate) {
        used.insert(candidate.clone());
        return candidate;
    }

    let mut index = 1usize;
    loop {
        let next = format!("{candidate}_{index}");
        if !used.contains(&next) {
            used.insert(next.clone());
            return next;
        }
        index += 1;
    }
}

fn storage_key_to_note_id(key: &str) -> String {
    let raw = key.strip_prefix("sticky_").unwrap_or(key);
    sanitize_note_id(raw)
}

fn normalize_record(record: &mut NoteRecord) {
    if record.id.trim().is_empty() {
        record.id = generate_note_id();
    } else {
        record.id = sanitize_note_id(&record.id);
    }

    if record.created_at.trim().is_empty() {
        record.created_at = now_iso();
    }
    if record.updated_at.trim().is_empty() {
        record.updated_at = record.created_at.clone();
    }

    if record.note.color.trim().is_empty() {
        record.note.color = default_note_color();
    }
    if record.note.width == 0 {
        record.note.width = default_note_width();
    }
    if record.note.height == 0 {
        record.note.height = default_note_height();
    }
    if record.note.zoom <= 0.0 {
        record.note.zoom = default_zoom();
    }

    match record.status {
        NoteStatus::Open => {
            record.closed_at = None;
            record.archived_at = None;
        }
        NoteStatus::Closed => {
            record.archived_at = None;
            if record.closed_at.is_none() {
                record.closed_at = Some(record.updated_at.clone());
            }
        }
        NoteStatus::Archived => {
            if record.archived_at.is_none() {
                record.archived_at = Some(record.updated_at.clone());
            }
        }
    }
}

fn deserialize_record(storage_key: &str, value: serde_json::Value) -> anyhow::Result<NoteRecord> {
    if let Ok(mut record) = serde_json::from_value::<NoteRecord>(value.clone()) {
        if record.id.trim().is_empty() {
            record.id = storage_key_to_note_id(storage_key);
        }
        normalize_record(&mut record);
        return Ok(record);
    }

    let mut note = serde_json::from_value::<Note>(value)?;
    if note.color.trim().is_empty() {
        note.color = default_note_color();
    }

    let now = now_iso();
    Ok(NoteRecord {
        id: storage_key_to_note_id(storage_key),
        status: NoteStatus::Open,
        created_at: now.clone(),
        updated_at: now,
        closed_at: None,
        archived_at: None,
        note,
    })
}

fn load_note_records(app: &AppHandle) -> anyhow::Result<Vec<NoteRecord>> {
    let store = app.store(NOTES_DATA)?;

    let mut used_ids = HashSet::new();
    let mut out = Vec::new();

    if let Some(val) = store.get("data") {
        let map = val
            .as_object()
            .context("json key 'data' contained a non-object")?;

        for (key, value) in map {
            let mut record = deserialize_record(key, value.clone())?;
            record.id = ensure_unique_id(record.id, &mut used_ids);
            out.push(record);
        }
    }

    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

fn save_note_records(app: &AppHandle, records: &[NoteRecord]) -> anyhow::Result<()> {
    let store = app.store(NOTES_DATA)?;

    let mut map = serde_json::Map::new();
    for record in records {
        map.insert(record.id.clone(), serde_json::to_value(record)?);
    }

    store.set("data", map);
    store.save()?;

    Ok(())
}

fn plain_text_from_quill_delta(contents: &str) -> String {
    let Ok(delta_value) = serde_json::from_str::<serde_json::Value>(contents) else {
        return contents.to_string();
    };

    let Some(ops) = delta_value.get("ops").and_then(|value| value.as_array()) else {
        return contents.to_string();
    };

    let mut out = String::new();
    for op in ops {
        if let Some(insert) = op.get("insert") {
            if let Some(text) = insert.as_str() {
                out.push_str(text);
            } else if insert.is_object() {
                out.push_str("\n[embedded content]\n");
            }
        }
    }

    out
}

fn note_to_markdown(record: &NoteRecord) -> String {
    let body = plain_text_from_quill_delta(&record.note.contents);

    format!(
        "---\nid: {}\nstatus: {:?}\ncreated_at: {}\nupdated_at: {}\nclosed_at: {}\narchived_at: {}\n---\n\n{}",
        record.id,
        record.status,
        record.created_at,
        record.updated_at,
        record.closed_at.clone().unwrap_or_default(),
        record.archived_at.clone().unwrap_or_default(),
        body.trim_end()
    )
}

pub fn notes_directory(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .context("Failed to get app data directory")?;

    let notes_dir = app_data_dir.join(MARKDOWN_NOTES_FOLDER);
    fs::create_dir_all(&notes_dir).context("Failed to create notes directory")?;

    Ok(notes_dir)
}

fn note_markdown_path(app: &AppHandle, note_id: &str) -> anyhow::Result<PathBuf> {
    let notes_dir = notes_directory(app)?;
    Ok(notes_dir.join(format!("{note_id}.md")))
}

fn sync_markdown_file(app: &AppHandle, record: &NoteRecord) -> anyhow::Result<()> {
    let path = note_markdown_path(app, &record.id)?;
    fs::write(path, note_to_markdown(record)).context("Failed to write markdown note")?;
    Ok(())
}

fn sync_all_markdown_files(app: &AppHandle, records: &[NoteRecord]) -> anyhow::Result<()> {
    let notes_dir = notes_directory(app)?;

    let mut expected_files = HashSet::new();
    for record in records {
        expected_files.insert(format!("{}.md", record.id));
        sync_markdown_file(app, record)?;
    }

    for entry in fs::read_dir(notes_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !expected_files.contains(file_name) {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

pub fn generate_note_id() -> String {
    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or_default();

    let sequence = NOTE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("n{micros}_{sequence}")
}

pub fn make_default_record(note_id: String) -> NoteRecord {
    let now = now_iso();
    NoteRecord {
        id: note_id,
        status: NoteStatus::Open,
        created_at: now.clone(),
        updated_at: now,
        closed_at: None,
        archived_at: None,
        note: Note::default(),
    }
}

pub fn create_backup(app: &AppHandle) -> anyhow::Result<()> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .context("Failed to get app data directory")?;

    let store_path = app_data_dir.join(NOTES_DATA);

    if !store_path.exists() {
        return Ok(());
    }

    let backup_dir = app_data_dir.join(BACKUP_FOLDER);
    fs::create_dir_all(&backup_dir).context("Failed to create backup directory")?;

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let backup_filename = format!("{}_{}", timestamp, NOTES_DATA);
    let backup_path = backup_dir.join(backup_filename);

    fs::copy(&store_path, &backup_path).context("Failed to create backup")?;

    log::info!("Created backup: {:?}", backup_path);

    cleanup_old_backups(&backup_dir)?;

    Ok(())
}

// Remove backups older than 30 days
fn cleanup_old_backups(backup_dir: &PathBuf) -> anyhow::Result<()> {
    let cutoff_date = Local::now() - Duration::days(30);

    let entries = fs::read_dir(backup_dir).context("Failed to read backup directory")?;

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

    let records = load_note_records(app)?;

    for record in records
        .iter()
        .filter(|record| record.status == NoteStatus::Open)
    {
        if let Err(e) = create_sticky(app, Some(record)) {
            log::error!("Error creating window with payload: {:#}", e);
        }
    }

    save_note_records(app, &records)?;
    sync_all_markdown_files(app, &records)?;

    Ok(())
}

pub fn save_sticky(app: &AppHandle, note_id: &str, note: Note) -> Result<(), anyhow::Error> {
    let mut records = load_note_records(app)?;
    let now = now_iso();
    let normalized_id = sanitize_note_id(note_id);

    if let Some(record) = records.iter_mut().find(|record| record.id == normalized_id) {
        record.note = note;
        record.status = NoteStatus::Open;
        record.updated_at = now;
        record.closed_at = None;
        record.archived_at = None;
        sync_markdown_file(app, record)?;
    } else {
        let mut record = make_default_record(normalized_id);
        record.note = note;
        sync_markdown_file(app, &record)?;
        records.push(record);
    }

    save_note_records(app, &records)?;

    Ok(())
}

pub fn mark_note_closed(app: &AppHandle, note_id: &str) -> Result<(), anyhow::Error> {
    let mut records = load_note_records(app)?;
    let now = now_iso();

    if let Some(record) = records.iter_mut().find(|record| record.id == note_id) {
        record.status = NoteStatus::Closed;
        record.closed_at = Some(now.clone());
        record.updated_at = now;
        sync_markdown_file(app, record)?;
        save_note_records(app, &records)?;
    }

    Ok(())
}

pub fn mark_note_archived(app: &AppHandle, note_id: &str) -> Result<(), anyhow::Error> {
    let mut records = load_note_records(app)?;
    let now = now_iso();

    if let Some(record) = records.iter_mut().find(|record| record.id == note_id) {
        record.status = NoteStatus::Archived;
        record.archived_at = Some(now.clone());
        record.closed_at = Some(now.clone());
        record.updated_at = now;
        sync_markdown_file(app, record)?;
        save_note_records(app, &records)?;
    }

    Ok(())
}

pub fn mark_note_open(app: &AppHandle, note_id: &str) -> Result<Option<NoteRecord>, anyhow::Error> {
    let mut records = load_note_records(app)?;
    let now = now_iso();

    let mut out = None;

    if let Some(record) = records.iter_mut().find(|record| record.id == note_id) {
        record.status = NoteStatus::Open;
        record.updated_at = now;
        record.closed_at = None;
        record.archived_at = None;
        sync_markdown_file(app, record)?;
        out = Some(record.clone());
        save_note_records(app, &records)?;
    }

    Ok(out)
}

pub fn delete_note(app: &AppHandle, note_id: &str) -> Result<(), anyhow::Error> {
    let mut records = load_note_records(app)?;
    records.retain(|record| record.id != note_id);

    save_note_records(app, &records)?;

    let path = note_markdown_path(app, note_id)?;
    if path.exists() {
        fs::remove_file(path).ok();
    }

    Ok(())
}

pub fn list_notes(app: &AppHandle) -> Result<Vec<NoteListItem>, anyhow::Error> {
    let mut records = load_note_records(app)?;
    records.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(records
        .into_iter()
        .map(|record| {
            let preview = plain_text_from_quill_delta(&record.note.contents)
                .trim()
                .replace('\n', " ")
                .chars()
                .take(160)
                .collect::<String>();

            NoteListItem {
                id: record.id,
                status: record.status,
                created_at: record.created_at,
                updated_at: record.updated_at,
                closed_at: record.closed_at,
                archived_at: record.archived_at,
                color: record.note.color,
                preview,
            }
        })
        .collect())
}

pub fn load_settings(app: &AppHandle) -> anyhow::Result<MenuSettings> {
    log::info!("Loading settings");

    let store = app.store(SETTINGS)?;

    let bring_to_front = store
        .get("bring_to_front")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let autostart = store
        .get("autostart")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    MenuSettings::new(app, bring_to_front, autostart)
}

pub fn save_settings(app: &AppHandle) -> anyhow::Result<()> {
    log::info!("Saving settings");

    let store = app.store(SETTINGS)?;
    let settings = app.state::<MenuSettings>();

    store.set("bring_to_front", settings.bring_to_front()?);

    Ok(())
}
