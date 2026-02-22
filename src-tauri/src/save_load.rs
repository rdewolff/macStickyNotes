use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration as StdDuration, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use chrono::{DateTime, Duration, Local, Utc};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tauri_plugin_log::log;
use tauri_plugin_store::StoreExt;

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    settings::MenuSettings,
    windows::{close_sticky_by_note_id, create_sticky},
};

const NOTES_DATA: &str = "save_data";
const BACKUP_FOLDER: &str = "backups";
const MARKDOWN_NOTES_FOLDER: &str = "notes";
const THEME_STYLESHEET_FILE: &str = "theme.css";
const SETTINGS: &str = "settings";
const NOTES_DIRECTORY_SETTING_KEY: &str = "notes_directory";
const DEFAULT_THEME_STYLESHEET: &str = r#"/* macStickyNotes theme.css
   Edit values below, then restart the app.
*/

:root {
  /* Palette used by the sticky note color picker */
  --sticky-color-1: #f9e7a7; /* Butter */
  --sticky-color-2: #bddcf6; /* Sky */
  --sticky-color-3: #bfe6bf; /* Mint */
  --sticky-color-4: #cfeee8; /* Seafoam */
  --sticky-color-5: #d6e8b6; /* Sage */
  --sticky-color-6: #edc2ce; /* Rose */
  --sticky-color-7: #d7c2e9; /* Lavender */
  --sticky-default-color: var(--sticky-color-1);

  /* Basic spacing / sizing */
  --sticky-editor-top-padding: 40px;
  --sticky-list-indent-left: 1.1em;
  --sticky-list-item-margin-y: 0.16em;
  --sticky-list-checkbox-offset-left: -1.25em;
  --sticky-list-checkbox-width: 1.05em;
  --sticky-titlebar-height: 30px;
  --sticky-titlebar-button-size: 28px;
  --sticky-corner-radius: 12px;
}
"#;

static NOTE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
static MARKDOWN_SYNC_SUPPRESS_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

pub struct NotesFolderWatcherState {
    watcher: Mutex<Option<RecommendedWatcher>>,
    debounce_generation: Arc<AtomicU64>,
}

impl Default for NotesFolderWatcherState {
    fn default() -> Self {
        Self {
            watcher: Mutex::new(None),
            debounce_generation: Arc::new(AtomicU64::new(0)),
        }
    }
}

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
    "#f9e7a7".to_string()
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

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn suppress_markdown_watch_events_for(duration: StdDuration) {
    let until = now_millis().saturating_add(duration.as_millis() as u64);
    MARKDOWN_SYNC_SUPPRESS_UNTIL_MS.store(until, Ordering::Relaxed);
}

fn markdown_watch_events_suppressed() -> bool {
    now_millis() < MARKDOWN_SYNC_SUPPRESS_UNTIL_MS.load(Ordering::Relaxed)
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

#[derive(Default)]
struct InlineSegment {
    text: String,
    attributes: serde_json::Map<String, serde_json::Value>,
}

#[derive(Default)]
struct DeltaLine {
    segments: Vec<InlineSegment>,
    block_attributes: serde_json::Map<String, serde_json::Value>,
}

fn bool_attr(attributes: &serde_json::Map<String, serde_json::Value>, key: &str) -> bool {
    attributes
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn u64_attr(attributes: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<u64> {
    attributes.get(key).and_then(serde_json::Value::as_u64)
}

fn str_attr<'a>(
    attributes: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<&'a str> {
    attributes.get(key).and_then(serde_json::Value::as_str)
}

fn render_plain_segments(segments: &[InlineSegment]) -> String {
    segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<String>()
}

fn render_inline_segment(segment: &InlineSegment) -> String {
    if segment.text.is_empty() {
        return String::new();
    }

    if bool_attr(&segment.attributes, "code") {
        return format!("`{}`", segment.text.replace('`', "\\`"));
    }

    let mut rendered = segment.text.clone();

    if let Some(link) = str_attr(&segment.attributes, "link") {
        rendered = format!("[{rendered}]({link})");
    }

    if bool_attr(&segment.attributes, "bold") {
        rendered = format!("**{rendered}**");
    }
    if bool_attr(&segment.attributes, "italic") {
        rendered = format!("*{rendered}*");
    }
    if bool_attr(&segment.attributes, "strike") {
        rendered = format!("~~{rendered}~~");
    }

    rendered
}

fn render_inline_segments(segments: &[InlineSegment]) -> String {
    segments
        .iter()
        .map(render_inline_segment)
        .collect::<Vec<_>>()
        .join("")
}

fn quill_delta_to_lines(contents: &str) -> Option<Vec<DeltaLine>> {
    let delta_value = serde_json::from_str::<serde_json::Value>(contents).ok()?;
    let ops = delta_value
        .get("ops")
        .and_then(serde_json::Value::as_array)?;

    let mut lines = Vec::new();
    let mut current_line = DeltaLine::default();

    for op in ops {
        let attributes = op
            .get("attributes")
            .and_then(serde_json::Value::as_object)
            .cloned()
            .unwrap_or_default();

        let Some(insert) = op.get("insert") else {
            continue;
        };

        if let Some(text) = insert.as_str() {
            let mut chunk_start = 0usize;

            for (index, ch) in text.char_indices() {
                if ch != '\n' {
                    continue;
                }

                if index > chunk_start {
                    current_line.segments.push(InlineSegment {
                        text: text[chunk_start..index].to_string(),
                        attributes: attributes.clone(),
                    });
                }

                lines.push(DeltaLine {
                    segments: std::mem::take(&mut current_line.segments),
                    block_attributes: attributes.clone(),
                });

                chunk_start = index + ch.len_utf8();
            }

            if chunk_start < text.len() {
                current_line.segments.push(InlineSegment {
                    text: text[chunk_start..].to_string(),
                    attributes,
                });
            }
        } else if insert.is_object() {
            current_line.segments.push(InlineSegment {
                text: "[embedded content]".to_string(),
                attributes: serde_json::Map::new(),
            });
        }
    }

    if !current_line.segments.is_empty() || lines.is_empty() {
        lines.push(DeltaLine {
            segments: current_line.segments,
            block_attributes: serde_json::Map::new(),
        });
    }

    Some(lines)
}

fn quill_delta_to_markdown(contents: &str) -> String {
    let Some(lines) = quill_delta_to_lines(contents) else {
        return contents.to_string();
    };

    let mut rendered = Vec::new();
    let mut ordered_counters: Vec<usize> = Vec::new();
    let mut in_code_block = false;

    for line in lines {
        if bool_attr(&line.block_attributes, "code-block") {
            if !in_code_block {
                rendered.push("```".to_string());
                in_code_block = true;
            }
            rendered.push(render_plain_segments(&line.segments));
            continue;
        }

        if in_code_block {
            rendered.push("```".to_string());
            in_code_block = false;
        }

        if let Some(level) = u64_attr(&line.block_attributes, "header") {
            ordered_counters.clear();
            let header_level = level.clamp(1, 6) as usize;
            let header_text = render_inline_segments(&line.segments).trim().to_string();
            if header_text.is_empty() {
                rendered.push("#".repeat(header_level));
            } else {
                rendered.push(format!("{} {header_text}", "#".repeat(header_level)));
            }
            continue;
        }

        if let Some(list_kind) = str_attr(&line.block_attributes, "list") {
            let indent = u64_attr(&line.block_attributes, "indent").unwrap_or(0) as usize;
            let indent_prefix = "  ".repeat(indent);
            let item_text = render_inline_segments(&line.segments).trim().to_string();

            match list_kind {
                "ordered" => {
                    if ordered_counters.len() <= indent {
                        ordered_counters.resize(indent + 1, 0);
                    }
                    ordered_counters.truncate(indent + 1);
                    ordered_counters[indent] += 1;
                    rendered.push(format!(
                        "{indent_prefix}{}. {item_text}",
                        ordered_counters[indent]
                    ));
                }
                "checked" => {
                    ordered_counters.clear();
                    rendered.push(format!("{indent_prefix}- [x] {item_text}"));
                }
                "unchecked" => {
                    ordered_counters.clear();
                    rendered.push(format!("{indent_prefix}- [ ] {item_text}"));
                }
                _ => {
                    ordered_counters.clear();
                    rendered.push(format!("{indent_prefix}- {item_text}"));
                }
            }

            continue;
        }

        ordered_counters.clear();

        if bool_attr(&line.block_attributes, "blockquote") {
            let quote_text = render_inline_segments(&line.segments).trim().to_string();
            if quote_text.is_empty() {
                rendered.push(">".to_string());
            } else {
                rendered.push(format!("> {quote_text}"));
            }
            continue;
        }

        rendered.push(render_inline_segments(&line.segments));
    }

    if in_code_block {
        rendered.push("```".to_string());
    }

    rendered.join("\n")
}

fn note_to_markdown(record: &NoteRecord) -> String {
    let body = quill_delta_to_markdown(&record.note.contents);

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

#[derive(Default)]
struct ImportedMarkdownNote {
    id: String,
    status: Option<NoteStatus>,
    contents: String,
}

#[derive(serde::Serialize, Clone)]
struct ExternalNoteUpdatePayload {
    contents: String,
    color: String,
    zoom: f64,
}

fn normalize_id_for_lookup(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim_matches('_').to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn parse_note_status(value: &str) -> Option<NoteStatus> {
    match value.trim().to_ascii_lowercase().as_str() {
        "open" => Some(NoteStatus::Open),
        "closed" => Some(NoteStatus::Closed),
        "archived" => Some(NoteStatus::Archived),
        _ => None,
    }
}

fn parse_frontmatter_and_body(raw: &str) -> (HashMap<String, String>, String) {
    let normalized = raw.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        return (HashMap::new(), normalized);
    };

    let frontmatter_end = rest
        .find("\n---\n")
        .map(|index| (index, 5usize))
        .or_else(|| rest.find("\n---").map(|index| (index, 4usize)));

    let Some((index, marker_len)) = frontmatter_end else {
        return (HashMap::new(), normalized);
    };

    let mut map = HashMap::new();
    let frontmatter = &rest[..index];
    for line in frontmatter.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        map.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
    }

    let body_start = index.saturating_add(marker_len);
    let mut body = if body_start <= rest.len() {
        rest[body_start..].to_string()
    } else {
        String::new()
    };
    body = body.trim_start_matches('\n').to_string();

    (map, body)
}

fn note_id_from_markdown_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let fallback = stem.rsplit('_').next().unwrap_or(stem);
    normalize_id_for_lookup(fallback)
}

fn make_delta_text_op(
    text: String,
    attributes: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    if attributes.is_empty() {
        serde_json::json!({ "insert": text })
    } else {
        serde_json::json!({ "insert": text, "attributes": attributes })
    }
}

fn parse_inline_markdown(text: &str) -> Vec<(String, serde_json::Map<String, serde_json::Value>)> {
    let mut out = Vec::new();
    let mut index = 0usize;

    while index < text.len() {
        let remaining = &text[index..];

        if let Some(link_body) = remaining.strip_prefix('[') {
            if let Some(label_end) = link_body.find("](") {
                let after_label = &link_body[label_end + 2..];
                if let Some(url_end) = after_label.find(')') {
                    let label = &link_body[..label_end];
                    let url = &after_label[..url_end];
                    if !label.is_empty() && !url.is_empty() {
                        let mut attrs = serde_json::Map::new();
                        attrs.insert(
                            "link".to_string(),
                            serde_json::Value::String(url.to_string()),
                        );
                        out.push((label.to_string(), attrs));
                        index += 1 + label_end + 2 + url_end + 1;
                        continue;
                    }
                }
            }
        }

        if let Some(italic_body) = remaining.strip_prefix('*') {
            if let Some(end) = italic_body.find('*') {
                let value = &italic_body[..end];
                if !value.is_empty() {
                    let mut attrs = serde_json::Map::new();
                    attrs.insert("italic".to_string(), serde_json::Value::Bool(true));
                    out.push((value.to_string(), attrs));
                    index += end + 2;
                    continue;
                }
            }
        }

        if let Some(bold_body) = remaining.strip_prefix("**") {
            if let Some(end) = bold_body.find("**") {
                let value = &bold_body[..end];
                if !value.is_empty() {
                    let mut attrs = serde_json::Map::new();
                    attrs.insert("bold".to_string(), serde_json::Value::Bool(true));
                    out.push((value.to_string(), attrs));
                    index += end + 4;
                    continue;
                }
            }
        }

        if let Some(strike_body) = remaining.strip_prefix("~~") {
            if let Some(end) = strike_body.find("~~") {
                let value = &strike_body[..end];
                if !value.is_empty() {
                    let mut attrs = serde_json::Map::new();
                    attrs.insert("strike".to_string(), serde_json::Value::Bool(true));
                    out.push((value.to_string(), attrs));
                    index += end + 4;
                    continue;
                }
            }
        }

        if let Some(code_body) = remaining.strip_prefix('`') {
            if let Some(end) = code_body.find('`') {
                let value = &code_body[..end];
                if !value.is_empty() {
                    let mut attrs = serde_json::Map::new();
                    attrs.insert("code".to_string(), serde_json::Value::Bool(true));
                    out.push((value.to_string(), attrs));
                    index += end + 2;
                    continue;
                }
            }
        }

        let mut next_break = remaining.len();
        for token in ['[', '*', '`', '~'] {
            if let Some(next) = remaining.find(token) {
                next_break = next_break.min(next);
            }
        }

        if next_break == 0 {
            out.push((remaining[..1].to_string(), serde_json::Map::new()));
            index += 1;
            continue;
        }

        out.push((remaining[..next_break].to_string(), serde_json::Map::new()));
        index += next_break;
    }

    out
}

fn parse_markdown_line(line: &str) -> (String, serde_json::Map<String, serde_json::Value>) {
    let mut block_attributes = serde_json::Map::new();
    let mut text = line.to_string();
    let trimmed = line.trim_start();
    let leading_spaces = line.len().saturating_sub(trimmed.len());
    let indent = (leading_spaces / 2) as u64;

    if let Some(rest) = trimmed.strip_prefix('>') {
        block_attributes.insert("blockquote".to_string(), serde_json::Value::Bool(true));
        text = rest.trim_start().to_string();
        return (text, block_attributes);
    }

    if trimmed.starts_with('#') {
        let level = trimmed.chars().take_while(|ch| *ch == '#').count();
        let content = trimmed[level..].trim_start();
        if level > 0 && level <= 6 && !content.is_empty() {
            block_attributes.insert(
                "header".to_string(),
                serde_json::Value::Number(level.into()),
            );
            return (content.to_string(), block_attributes);
        }
    }

    if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        block_attributes.insert(
            "list".to_string(),
            serde_json::Value::String("unchecked".to_string()),
        );
        if indent > 0 {
            block_attributes.insert(
                "indent".to_string(),
                serde_json::Value::Number(indent.into()),
            );
        }
        return (rest.to_string(), block_attributes);
    }

    if let Some(rest) = trimmed
        .strip_prefix("- [x] ")
        .or_else(|| trimmed.strip_prefix("- [X] "))
    {
        block_attributes.insert(
            "list".to_string(),
            serde_json::Value::String("checked".to_string()),
        );
        if indent > 0 {
            block_attributes.insert(
                "indent".to_string(),
                serde_json::Value::Number(indent.into()),
            );
        }
        return (rest.to_string(), block_attributes);
    }

    if let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
    {
        block_attributes.insert(
            "list".to_string(),
            serde_json::Value::String("bullet".to_string()),
        );
        if indent > 0 {
            block_attributes.insert(
                "indent".to_string(),
                serde_json::Value::Number(indent.into()),
            );
        }
        return (rest.to_string(), block_attributes);
    }

    if let Some(dot) = trimmed.find(". ") {
        let prefix = &trimmed[..dot];
        if !prefix.is_empty() && prefix.chars().all(|ch| ch.is_ascii_digit()) {
            block_attributes.insert(
                "list".to_string(),
                serde_json::Value::String("ordered".to_string()),
            );
            if indent > 0 {
                block_attributes.insert(
                    "indent".to_string(),
                    serde_json::Value::Number(indent.into()),
                );
            }
            return (trimmed[dot + 2..].to_string(), block_attributes);
        }
    }

    (text, block_attributes)
}

fn markdown_to_quill_delta(markdown: &str) -> String {
    let normalized = markdown.replace("\r\n", "\n");
    if normalized.trim().is_empty() {
        return String::new();
    }

    let mut ops = Vec::new();
    let mut in_code_block = false;

    for line in normalized.split('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            ops.push(make_delta_text_op(line.to_string(), serde_json::Map::new()));
            let mut newline_attributes = serde_json::Map::new();
            newline_attributes.insert("code-block".to_string(), serde_json::Value::Bool(true));
            ops.push(make_delta_text_op("\n".to_string(), newline_attributes));
            continue;
        }

        let (text, block_attributes) = parse_markdown_line(line);
        let inline_segments = parse_inline_markdown(&text);
        for (segment, attrs) in inline_segments {
            if !segment.is_empty() {
                ops.push(make_delta_text_op(segment, attrs));
            }
        }

        ops.push(make_delta_text_op("\n".to_string(), block_attributes));
    }

    if ops.is_empty() {
        return String::new();
    }

    serde_json::to_string(&serde_json::json!({ "ops": ops })).unwrap_or_default()
}

fn parse_markdown_note(path: &Path, raw: &str) -> Option<ImportedMarkdownNote> {
    let (frontmatter, body) = parse_frontmatter_and_body(raw);

    let note_id = frontmatter
        .get("id")
        .and_then(|value| normalize_id_for_lookup(value))
        .or_else(|| note_id_from_markdown_path(path))?;

    let status = frontmatter
        .get("status")
        .and_then(|value| parse_note_status(value));

    Some(ImportedMarkdownNote {
        id: note_id,
        status,
        contents: markdown_to_quill_delta(&body),
    })
}

fn is_markdown_note_path(path: &Path) -> bool {
    if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
        return false;
    }

    path.file_name().and_then(|name| name.to_str()) != Some(THEME_STYLESHEET_FILE)
}

fn emit_external_note_update(app: &AppHandle, record: &NoteRecord) {
    let window_label = format!("sticky_{}", record.id);
    let payload = ExternalNoteUpdatePayload {
        contents: record.note.contents.clone(),
        color: record.note.color.clone(),
        zoom: record.note.zoom,
    };

    if let Some(window) = app.get_webview_window(&window_label) {
        let _ = window.emit("external_note_update", payload);
    }
}

fn sync_from_markdown_directory_internal(
    app: &AppHandle,
    update_windows: bool,
) -> anyhow::Result<bool> {
    let notes_dir = notes_directory(app)?;
    let mut records = load_note_records(app)?;

    let mut imported_by_id = HashMap::<String, ImportedMarkdownNote>::new();
    for entry in fs::read_dir(notes_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !is_markdown_note_path(&path) {
            continue;
        }

        match fs::read_to_string(&path) {
            Ok(raw) => {
                if let Some(imported) = parse_markdown_note(&path, &raw) {
                    imported_by_id.insert(imported.id.clone(), imported);
                }
            }
            Err(error) => {
                log::warn!("Failed reading markdown note {:?}: {}", path, error);
            }
        }
    }

    let imported_ids = imported_by_id.keys().cloned().collect::<HashSet<String>>();
    let mut changed = false;
    let mut open_changed_ids = Vec::<String>::new();
    let mut open_created_ids = Vec::<String>::new();
    let mut close_ids = Vec::<String>::new();

    for record in &mut records {
        if let Some(imported) = imported_by_id.get(&record.id) {
            let desired_status = imported.status.unwrap_or(record.status);

            if record.note.contents != imported.contents || record.status != desired_status {
                record.note.contents = imported.contents.clone();
                record.status = desired_status;
                record.updated_at = now_iso();
                normalize_record(record);
                changed = true;

                if record.status == NoteStatus::Open {
                    open_changed_ids.push(record.id.clone());
                } else {
                    close_ids.push(record.id.clone());
                }
            }
        }
    }

    let mut retained = Vec::with_capacity(records.len());
    for record in records {
        if imported_ids.contains(&record.id) {
            retained.push(record);
        } else {
            changed = true;
            close_ids.push(record.id.clone());
        }
    }
    let mut records = retained;

    for imported in imported_by_id.into_values() {
        if records.iter().any(|record| record.id == imported.id) {
            continue;
        }

        let mut record = make_default_record(imported.id);
        record.note.contents = imported.contents;
        record.status = imported.status.unwrap_or(NoteStatus::Open);
        normalize_record(&mut record);
        if record.status == NoteStatus::Open {
            open_created_ids.push(record.id.clone());
        }
        records.push(record);
        changed = true;
    }

    if !changed {
        return Ok(false);
    }

    save_note_records(app, &records)?;

    if update_windows {
        for note_id in close_ids {
            let _ = close_sticky_by_note_id(app, &note_id);
        }

        for note_id in open_created_ids {
            if let Some(record) = records.iter().find(|record| record.id == note_id) {
                let _ = create_sticky(app, Some(record));
            }
        }

        for note_id in open_changed_ids {
            if let Some(record) = records.iter().find(|record| record.id == note_id) {
                let window_label = format!("sticky_{}", record.id);
                if app.get_webview_window(&window_label).is_some() {
                    emit_external_note_update(app, record);
                } else {
                    let _ = create_sticky(app, Some(record));
                }
            }
        }
    }

    let _ = app.emit("notes_changed", ());
    Ok(true)
}

pub fn sync_from_markdown_directory(app: &AppHandle) -> anyhow::Result<bool> {
    sync_from_markdown_directory_internal(app, true)
}

pub fn restart_notes_directory_watcher(app: &AppHandle) -> anyhow::Result<()> {
    let notes_dir = notes_directory(app)?;
    let state = app.state::<NotesFolderWatcherState>();
    let generation = state.debounce_generation.clone();
    let app_handle = app.clone();
    let watched_dir = notes_dir.clone();

    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let event = match result {
            Ok(event) => event,
            Err(error) => {
                log::warn!("Notes watcher error: {}", error);
                return;
            }
        };

        if markdown_watch_events_suppressed() {
            return;
        }

        let relevant = event.paths.iter().any(|path| {
            is_markdown_note_path(path.as_path())
                || path == &watched_dir
                || path.starts_with(&watched_dir)
        });
        if !relevant {
            return;
        }

        let next = generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation_clone = generation.clone();
        let app_clone = app_handle.clone();
        tauri::async_runtime::spawn_blocking(move || {
            std::thread::sleep(StdDuration::from_millis(300));
            if generation_clone.load(Ordering::SeqCst) != next {
                return;
            }

            if markdown_watch_events_suppressed() {
                return;
            }

            if let Err(error) = sync_from_markdown_directory(&app_clone) {
                log::warn!("Failed syncing markdown directory: {}", error);
            }
        });
    })?;

    watcher.configure(Config::default())?;
    watcher.watch(&notes_dir, RecursiveMode::NonRecursive)?;

    let mut guard = state
        .watcher
        .lock()
        .map_err(|_| anyhow::anyhow!("failed to lock notes watcher state"))?;
    *guard = Some(watcher);

    Ok(())
}

fn default_notes_directory(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .context("Failed to get app data directory")?;

    Ok(app_data_dir.join(MARKDOWN_NOTES_FOLDER))
}

pub fn notes_directory(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let store = app.store(SETTINGS)?;
    let configured = store
        .get(NOTES_DIRECTORY_SETTING_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from);

    let notes_dir = configured.unwrap_or(default_notes_directory(app)?);
    fs::create_dir_all(&notes_dir).context("Failed to create notes directory")?;

    Ok(notes_dir)
}

fn file_date_prefix(created_at: &str) -> String {
    DateTime::parse_from_rfc3339(created_at)
        .map(|date| date.with_timezone(&Local).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| Local::now().format("%Y-%m-%d").to_string())
}

fn slugify_filename_part(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            slug.push('_');
            previous_was_separator = true;
        }
    }

    slug = slug.trim_matches('_').to_string();
    slug.chars().take(48).collect()
}

fn title_from_record(record: &NoteRecord) -> String {
    plain_text_from_quill_delta(&record.note.contents)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_string()
}

fn markdown_filename(record: &NoteRecord) -> String {
    let date_prefix = file_date_prefix(&record.created_at);
    let title_part = slugify_filename_part(&title_from_record(record));
    let id_part = sanitize_note_id(&record.id).to_ascii_lowercase();

    if title_part.is_empty() {
        format!("{date_prefix}_{id_part}.md")
    } else {
        format!("{date_prefix}_{title_part}_{id_part}.md")
    }
}

fn note_markdown_path(app: &AppHandle, record: &NoteRecord) -> anyhow::Result<PathBuf> {
    let notes_dir = notes_directory(app)?;
    Ok(notes_dir.join(markdown_filename(record)))
}

fn theme_stylesheet_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let notes_dir = notes_directory(app)?;
    Ok(notes_dir.join(THEME_STYLESHEET_FILE))
}

pub fn load_theme_stylesheet(app: &AppHandle) -> anyhow::Result<String> {
    let path = theme_stylesheet_path(app)?;
    if !path.exists() {
        fs::write(&path, DEFAULT_THEME_STYLESHEET).context("Failed to create theme stylesheet")?;
    }

    fs::read_to_string(path).context("Failed to read theme stylesheet")
}

fn sync_markdown_file(app: &AppHandle, record: &NoteRecord) -> anyhow::Result<()> {
    let path = note_markdown_path(app, record)?;
    suppress_markdown_watch_events_for(StdDuration::from_millis(1200));
    fs::write(path, note_to_markdown(record)).context("Failed to write markdown note")?;
    Ok(())
}

fn sync_all_markdown_files(app: &AppHandle, records: &[NoteRecord]) -> anyhow::Result<()> {
    let notes_dir = notes_directory(app)?;

    let mut expected_files = HashSet::new();
    for record in records {
        expected_files.insert(markdown_filename(record));
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

        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }

        if !expected_files.contains(file_name) {
            suppress_markdown_watch_events_for(StdDuration::from_millis(1200));
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

pub fn get_notes_directory_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    notes_directory(app)
}

pub fn set_notes_directory_path(app: &AppHandle, path: PathBuf) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(&path).context("Failed to create notes directory")?;
    let resolved = fs::canonicalize(&path).unwrap_or(path);

    let store = app.store(SETTINGS)?;
    store.set(
        NOTES_DIRECTORY_SETTING_KEY,
        resolved.to_string_lossy().to_string(),
    );
    store.save()?;

    let records = load_note_records(app)?;
    sync_all_markdown_files(app, &records)?;

    Ok(resolved)
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
    let _ = sync_from_markdown_directory_internal(app, false)?;

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
    let mut stale_markdown_file: Option<PathBuf> = None;

    if let Some(record) = records.iter_mut().find(|record| record.id == normalized_id) {
        let previous_filename = markdown_filename(record);
        record.note = note;
        record.status = NoteStatus::Open;
        record.updated_at = now;
        record.closed_at = None;
        record.archived_at = None;
        sync_markdown_file(app, record)?;

        let next_filename = markdown_filename(record);
        if previous_filename != next_filename {
            stale_markdown_file = Some(notes_directory(app)?.join(previous_filename));
        }
    } else {
        let mut record = make_default_record(normalized_id);
        record.note = note;
        sync_markdown_file(app, &record)?;
        records.push(record);
    }

    save_note_records(app, &records)?;
    if let Some(path) = stale_markdown_file {
        let _ = fs::remove_file(path);
    }

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
    let mut removed_record = None;

    for record in &records {
        if record.id == note_id {
            removed_record = Some(record.clone());
            break;
        }
    }

    records.retain(|record| record.id != note_id);

    save_note_records(app, &records)?;

    if let Some(record) = removed_record {
        let path = note_markdown_path(app, &record)?;
        if path.exists() {
            let _ = fs::remove_file(path);
        }
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

#[cfg(test)]
mod tests {
    use super::quill_delta_to_markdown;

    #[test]
    fn converts_rich_text_blocks_to_markdown() {
        let delta = r#"{"ops":[{"insert":"Title"},{"insert":"\n","attributes":{"header":1}},{"insert":"Bold "},{"insert":"text","attributes":{"bold":true}},{"insert":" and "},{"insert":"italic","attributes":{"italic":true}},{"insert":"\n"},{"insert":"Done item"},{"insert":"\n","attributes":{"list":"checked"}},{"insert":"Todo item"},{"insert":"\n","attributes":{"list":"unchecked"}}]}"#;

        let markdown = quill_delta_to_markdown(delta);

        assert_eq!(
            markdown,
            "# Title\nBold **text** and *italic*\n- [x] Done item\n- [ ] Todo item"
        );
    }

    #[test]
    fn converts_code_blocks_to_fenced_markdown() {
        let delta = r#"{"ops":[{"insert":"let x = 1;"},{"insert":"\n","attributes":{"code-block":true}},{"insert":"let y = 2;"},{"insert":"\n","attributes":{"code-block":true}}]}"#;

        let markdown = quill_delta_to_markdown(delta);

        assert_eq!(markdown, "```\nlet x = 1;\nlet y = 2;\n```");
    }

    #[test]
    fn returns_original_contents_when_not_a_delta() {
        let raw = "# already markdown";
        assert_eq!(quill_delta_to_markdown(raw), raw);
    }
}
