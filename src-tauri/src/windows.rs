use anyhow::{bail, Context};
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalPosition, Manager, PhysicalPosition, PhysicalSize,
    WebviewUrl, WebviewWindow, WindowEvent,
};
use tauri_plugin_log::log;

use crate::save_load::{
    generate_note_id, make_default_record, mark_note_closed, save_sticky, Note, NoteRecord,
};

const GAP: i32 = 20;
const VISIBLE_PADDING: f64 = 32.0;
const STICKY_WINDOW_PREFIX: &str = "sticky_";
pub const MANAGER_WINDOW_LABEL: &str = "manager";

pub fn is_sticky_window_label(label: &str) -> bool {
    label.starts_with(STICKY_WINDOW_PREFIX)
}

pub fn note_id_from_label(label: &str) -> Option<String> {
    label
        .strip_prefix(STICKY_WINDOW_PREFIX)
        .map(str::to_string)
        .filter(|id| !id.is_empty())
}

fn sticky_label(note_id: &str) -> String {
    format!("{STICKY_WINDOW_PREFIX}{note_id}")
}

fn get_focused_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| is_sticky_window_label(label))
        .find(|(_, window)| window.is_focused().unwrap_or(false))
        .map(|(_, window)| window)
}

fn get_position_and_size(
    window: &WebviewWindow,
) -> Result<(PhysicalPosition<i32>, PhysicalSize<u32>), anyhow::Error> {
    let window_position = window.outer_position().context(format!(
        "Could not get position of window: {}",
        window.label()
    ))?;
    let window_size = window
        .outer_size()
        .context(format!("Could not get size of window: {}", window.label()))?;
    Ok((window_position, window_size))
}

fn window_overlap(start_1: i32, len_1: i32, start_2: i32, len_2: i32) -> bool {
    let end_1 = start_1 + len_1;
    let end_2 = start_2 + len_2;

    let overlap_start = std::cmp::max(start_1, start_2);
    let overlap_end = std::cmp::min(end_1, end_2);
    overlap_end - overlap_start > GAP
}

pub fn snap_window(
    app: &AppHandle,
    direction: Direction,
    partial: bool,
) -> Result<(), anyhow::Error> {
    log::debug!("Snapping window {:?}", direction);

    let window = get_focused_window(app).context("No window currently focused")?;
    let (window_position, window_size) = get_position_and_size(&window)?;

    let primary_monitor = app
        .primary_monitor()
        .context("could not get primary monitor")?
        .context("no primary monitor")?;

    let active_monitor = app
        .cursor_position()
        .map(|p| p.to_logical(primary_monitor.scale_factor()))
        .and_then(|p| app.monitor_from_point(p.x, p.y))
        .context("could not get cursor position")?
        .context("could not get monitor from cursor position")?;

    let current_monitor = window
        .current_monitor()
        .context(format!(
            "could not find monitor for window to be positioned: {}",
            window.label()
        ))?
        .context("window to be positioned is hidden or otherwise has no display")?;

    if current_monitor.name() != active_monitor.name() {
        window.set_position(
            (PhysicalPosition {
                x: active_monitor.position().x + GAP,
                y: active_monitor.position().y + GAP,
            })
            .to_logical::<i32>(active_monitor.scale_factor()),
        )?;
        return Ok(());
    }

    let other_windows = app
        .webview_windows()
        .into_iter()
        .filter(|(label, wind)| is_sticky_window_label(label) && *wind != window)
        .filter_map(|(_, wind)| get_position_and_size(&wind).ok());

    let viable_edges: Box<dyn Iterator<Item = i32>> =
        if partial {
            match direction {
                Direction::Left => Box::new(other_windows.flat_map(|(position, size)| {
                    [position.x + size.width as i32 + GAP, position.x]
                })),
                Direction::Up => Box::new(other_windows.flat_map(|(position, size)| {
                    [position.y + size.height as i32 + GAP, position.y]
                })),
                Direction::Right => Box::new(other_windows.flat_map(|(position, size)| {
                    [
                        (position.x + size.width as i32) - window_size.width as i32,
                        position.x - (window_size.width as i32 + GAP),
                    ]
                })),
                Direction::Down => Box::new(other_windows.flat_map(|(position, size)| {
                    [
                        (position.y + size.height as i32) - window_size.height as i32,
                        position.y - (window_size.height as i32 + GAP),
                    ]
                })),
            }
        } else {
            match direction {
                Direction::Left => Box::new(other_windows.filter_map(|(position, size)| {
                    if window_overlap(
                        position.y,
                        size.height as i32,
                        window_position.y,
                        window_size.height as i32,
                    ) {
                        Some(position.x + size.width as i32 + GAP)
                    } else {
                        None
                    }
                })),
                Direction::Up => Box::new(other_windows.filter_map(|(position, size)| {
                    if window_overlap(
                        position.x,
                        size.width as i32,
                        window_position.x,
                        window_size.width as i32,
                    ) {
                        Some(position.y + size.height as i32 + GAP)
                    } else {
                        None
                    }
                })),
                Direction::Right => Box::new(other_windows.filter_map(|(position, size)| {
                    if window_overlap(
                        position.y,
                        size.height as i32,
                        window_position.y,
                        window_size.height as i32,
                    ) {
                        Some(position.x - (window_size.width as i32 + GAP))
                    } else {
                        None
                    }
                })),
                Direction::Down => Box::new(other_windows.filter_map(|(position, size)| {
                    if window_overlap(
                        position.x,
                        size.width as i32,
                        window_position.x,
                        window_size.width as i32,
                    ) {
                        Some(position.y - (window_size.height as i32 + GAP))
                    } else {
                        None
                    }
                })),
            }
        };

    let position = match direction {
        Direction::Left => PhysicalPosition {
            x: viable_edges
                .filter(|edge| *edge < window_position.x)
                .max()
                .unwrap_or(current_monitor.position().x + GAP),
            y: window_position.y,
        },
        Direction::Up => PhysicalPosition {
            x: window_position.x,
            y: viable_edges
                .filter(|edge| *edge < window_position.y)
                .max()
                .unwrap_or(current_monitor.position().y + GAP),
        },
        Direction::Right => PhysicalPosition {
            x: viable_edges
                .filter(|edge| *edge > window_position.x)
                .min()
                .unwrap_or(
                    ((current_monitor.position().x + current_monitor.size().width as i32)
                        - window_size.width as i32)
                        - GAP,
                ),
            y: window_position.y,
        },
        Direction::Down => PhysicalPosition {
            x: window_position.x,
            y: viable_edges
                .filter(|edge| *edge > window_position.y)
                .min()
                .unwrap_or(
                    ((current_monitor.position().y + current_monitor.size().height as i32)
                        - window_size.height as i32)
                        - GAP,
                ),
        },
    };

    window.set_position(position)?;
    Ok(())
}

fn logical_monitor_bounds(monitor: &tauri::Monitor) -> (f64, f64, f64, f64) {
    let scale = monitor.scale_factor();
    let position = monitor.position().to_logical::<f64>(scale);
    let size = monitor.size().to_logical::<f64>(scale);
    (
        position.x,
        position.y,
        position.x + size.width,
        position.y + size.height,
    )
}

fn monitor_contains_note(
    monitor: &tauri::Monitor,
    note_x: f64,
    note_y: f64,
    note_width: f64,
    note_height: f64,
) -> bool {
    let (left, top, right, bottom) = logical_monitor_bounds(monitor);
    let note_right = note_x + note_width;
    let note_bottom = note_y + note_height;

    !(note_right < left || note_x > right || note_bottom < top || note_y > bottom)
}

fn clamp_note_within_bounds(
    note_x: f64,
    note_y: f64,
    note_width: f64,
    note_height: f64,
    bounds: (f64, f64, f64, f64),
) -> (f64, f64) {
    let (left, top, right, bottom) = bounds;
    let avail_width = (right - left).max(0.0);
    let avail_height = (bottom - top).max(0.0);

    let width = note_width.min(avail_width);
    let height = note_height.min(avail_height);

    let min_x = left + VISIBLE_PADDING;
    let max_x = right - width - VISIBLE_PADDING;
    let min_y = top + VISIBLE_PADDING;
    let max_y = bottom - height - VISIBLE_PADDING;

    let x = if min_x <= max_x {
        note_x.clamp(min_x, max_x)
    } else {
        left
    };

    let y = if min_y <= max_y {
        note_y.clamp(min_y, max_y)
    } else {
        top
    };

    (x, y)
}

fn ensure_note_visible(app: &AppHandle, note: &Note) -> (f64, f64) {
    let note_x = note.x as f64;
    let note_y = note.y as f64;
    let note_width = note.width as f64;
    let note_height = note.height as f64;

    let monitors = match app.available_monitors() {
        Ok(monitors) if !monitors.is_empty() => monitors,
        _ => return (note_x, note_y),
    };

    if monitors
        .iter()
        .any(|monitor| monitor_contains_note(monitor, note_x, note_y, note_width, note_height))
    {
        return (note_x, note_y);
    }

    let target_bounds = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| logical_monitor_bounds(&monitor))
        .or_else(|| {
            monitors
                .first()
                .map(|monitor| logical_monitor_bounds(monitor))
        });

    if let Some(bounds) = target_bounds {
        return clamp_note_within_bounds(note_x, note_y, note_width, note_height, bounds);
    }

    (note_x, note_y)
}

pub fn create_sticky(
    app: &AppHandle,
    payload: Option<&NoteRecord>,
) -> Result<WebviewWindow, anyhow::Error> {
    log::debug!("Creating new sticky window");

    let mut record = if let Some(record) = payload {
        record.clone()
    } else {
        let note_id = generate_note_id();
        let record = make_default_record(note_id);
        save_sticky(app, &record.id, record.note.clone())?;
        record
    };

    let (initial_x, initial_y) = ensure_note_visible(app, &record.note);
    let corrected_x = initial_x.round() as i32;
    let corrected_y = initial_y.round() as i32;

    if record.note.x != corrected_x || record.note.y != corrected_y {
        record.note.x = corrected_x;
        record.note.y = corrected_y;
        save_sticky(app, &record.id, record.note.clone())?;
    }

    let label = sticky_label(&record.id);

    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.set_position(LogicalPosition::new(initial_x, initial_y));
        let _ = existing.unminimize();
        let _ = existing.show();
        existing.set_focus().ok();
        return Ok(existing);
    }

    let init_script = format!(
        r#"
            window.__STICKY_INIT__ = {};
        "#,
        serde_json::to_string(&record)?
    );

    let mut builder =
        tauri::WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
            .decorations(false)
            .transparent(true)
            .resizable(true)
            .visible(true)
            .accept_first_mouse(true)
            .initialization_script(init_script)
            .inner_size(record.note.width as f64, record.note.height as f64)
            .always_on_top(record.note.always_on_top);

    builder = builder.position(initial_x, initial_y);

    let window = builder.build().context("Could not create sticky window")?;
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    let app_clone = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { .. } = event {
            let _ = cycle_focus(&app_clone, false);
        }
    });

    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWindow;

        let ns_window_ptr = window.ns_window().unwrap();
        unsafe {
            use objc2_app_kit::NSWindowCollectionBehavior;

            let ns_window = &mut *(ns_window_ptr as *mut NSWindow);
            ns_window.setCollectionBehavior(
                NSWindowCollectionBehavior::IgnoresCycle | NSWindowCollectionBehavior::Transient,
            );
            ns_window.setHasShadow(true);
        }
    }

    Ok(window)
}

pub fn open_note_manager(app: &AppHandle) -> Result<(), anyhow::Error> {
    if let Some(window) = app.get_webview_window(MANAGER_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        window.set_focus()?;
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(
        app,
        MANAGER_WINDOW_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Notes Manager")
    .decorations(true)
    .transparent(false)
    .resizable(true)
    .visible(true)
    .inner_size(760.0, 560.0)
    .min_inner_size(620.0, 420.0)
    .initialization_script("window.__STICKY_MANAGER__ = true;")
    .build()
    .context("Could not create note manager window")?;

    Ok(())
}

pub fn close_sticky(app: &AppHandle) -> Result<(), anyhow::Error> {
    if let Some(window) = get_focused_window(app) {
        let note_id = note_id_from_label(window.label()).context("Missing note id for window")?;
        window.close()?;
        mark_note_closed(app, &note_id)?;
        Ok(())
    } else {
        bail!("No window currently focused!")
    }
}

pub fn close_sticky_by_note_id(app: &AppHandle, note_id: &str) -> Result<(), anyhow::Error> {
    let label = sticky_label(note_id);
    if let Some(window) = app.get_webview_window(&label) {
        window.close()?;
    }
    Ok(())
}

pub fn sorted_windows(app: &AppHandle) -> Vec<WebviewWindow> {
    let mut positions: Vec<_> = app
        .webview_windows()
        .into_iter()
        .filter(|(label, _)| is_sticky_window_label(label))
        .filter_map(|(_label, w)| get_position_and_size(&w).ok().map(|(p, _)| (p, w)))
        .collect();

    positions.sort_by_key(|(p, _)| *p);

    positions.into_iter().map(|(_, w)| w).collect()
}

pub fn cycle_focus(app: &AppHandle, reverse: bool) -> Result<(), anyhow::Error> {
    let mut sorted_windows = sorted_windows(app);
    if reverse {
        sorted_windows.reverse();
    }

    let focused_index = sorted_windows
        .iter()
        .position(|w| w.is_focused().unwrap_or(false))
        .context("No window currently focused")?;

    let next_window_index = (focused_index + 1) % sorted_windows.len();

    sorted_windows[next_window_index]
        .set_focus()
        .context("Could not focus window")
}

pub fn fit_text(app: &AppHandle) -> Result<(), anyhow::Error> {
    app.webview_windows()
        .into_iter()
        .for_each(|(label, window)| {
            if is_sticky_window_label(&label) && window.is_focused().unwrap_or(false) {
                log::info!("emitting fit_text to window {}", label);
                let _ = window.emit_to(EventTarget::webview_window(label), "fit_text", {});
            }
        });

    Ok(())
}

pub fn set_color(app: &AppHandle, index: u8) -> Result<(), anyhow::Error> {
    app.webview_windows()
        .into_iter()
        .for_each(|(label, window)| {
            if is_sticky_window_label(&label) && window.is_focused().unwrap_or(false) {
                log::info!("emitting set color to window {}", label);
                let _ = window.emit_to(EventTarget::webview_window(label), "set_color", index);
            }
        });

    Ok(())
}

pub fn reset_note_positions(app: &AppHandle) -> anyhow::Result<()> {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| is_sticky_window_label(label))
        .map(|(_, window)| {
            window
                .set_position(PhysicalPosition { x: 0, y: 0 })
                .context("could not set note position")
        })
        .collect::<Result<(), anyhow::Error>>()
}

pub fn emit_to_focused(app: &AppHandle, event: &str, payload: &str) -> anyhow::Result<()> {
    let window = get_focused_window(app).context("No window currently focused")?;
    window.emit_to(
        EventTarget::webview_window(window.label().to_string()),
        event,
        payload.to_string(),
    )?;
    Ok(())
}

pub fn set_always_on_top(app: &AppHandle, always_on_top: bool) -> anyhow::Result<()> {
    if let Some(window) = get_focused_window(app) {
        window
            .set_always_on_top(always_on_top)
            .context("Could not set window always on top")
    } else {
        bail!("No window currently focused!")
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
