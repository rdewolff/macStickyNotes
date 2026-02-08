use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{bail, Context};
use tauri::{
    AppHandle, Emitter, EventTarget, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent,
};
use tauri_plugin_log::log;

use crate::save_load::{save_sticky, Note};

const GAP: i32 = 20;

static WINDOW_ID: AtomicU32 = AtomicU32::new(0);

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

fn get_focused_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .find(|(_, window)| window.is_focused().unwrap_or(false))
        .map(|(_label, window)| window)
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

    let primary_monitor = app.primary_monitor().context("could not get primary monitor")?.context("no primary monitor")?;

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
                y: active_monitor.position().y + GAP
            }).to_logical::<i32>(active_monitor.scale_factor())
        )?;
        return Ok(())
    }

    let other_windows = app
        .webview_windows()
        .into_iter()
        .filter(|(_, wind)| *wind != window)
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
                .filter(|edge| *edge < window_position.x as i32)
                .max()
                .unwrap_or( current_monitor.position().x + GAP),
            y: window_position.y,
        },
        Direction::Up => PhysicalPosition {
            x: window_position.x,
            y: viable_edges
                .filter(|edge| *edge < window_position.y as i32)
                .max()
                .unwrap_or(current_monitor.position().y + GAP),
        },
        Direction::Right => PhysicalPosition {
            x: viable_edges
                .filter(|edge| *edge > window_position.x as i32)
                .min()
                .unwrap_or(((current_monitor.position().x + current_monitor.size().width as i32) - window_size.width as i32) - GAP),
            y: window_position.y,
        },
        Direction::Down => PhysicalPosition {
            x: window_position.x,
            y: viable_edges
                .filter(|edge| *edge > window_position.y as i32)
                .min()
                .unwrap_or(((current_monitor.position().y + current_monitor.size().height as i32) - window_size.height as i32) - GAP),
        },
    };

    window.set_position(position)?;
    Ok(())
}

pub fn create_sticky(app: &AppHandle, payload: Option<&Note>) -> Result<WebviewWindow, anyhow::Error> {
    log::debug!("Creating new sticky window");
    let label = format!("sticky_{}", WINDOW_ID.fetch_add(1, Ordering::Relaxed));

    let mut builder =
        tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App("index.html".into()))
            .decorations(false)
            .transparent(true)
            .resizable(true)
            .visible(true)
            .accept_first_mouse(true)
            .inner_size(300.0, 250.0);

    if let Some(note) = payload {
        let init_script = format!(r#"
            window.__STICKY_INIT__ = {}
        "#,
            serde_json::to_string(note)?
        );
        
        builder = builder
            .initialization_script(init_script)
            .inner_size(note.width as f64, note.height as f64)
            .always_on_top(note.always_on_top);

        if app.monitor_from_point(note.x as f64, note.y as f64)?.is_some() {
            builder = builder.position(note.x as f64, note.y as f64);
        } else {
            builder = builder.position(0., 0.);
        }
    }

    let window = builder.build().context("Could not create sticky window")?;
    let app_clone = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { .. } => {
            let _ = cycle_focus(&app_clone, false);
        }
        _ => {}
    });

    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWindow;

        let ns_window_ptr = window.ns_window().unwrap();
        unsafe {
            use objc2_app_kit::NSWindowCollectionBehavior;

            let ns_window = &mut *(ns_window_ptr as *mut NSWindow);
            ns_window.setCollectionBehavior(NSWindowCollectionBehavior::IgnoresCycle | NSWindowCollectionBehavior::Transient);
            ns_window.setHasShadow(true);
        }
    }

    Ok(window)
}

pub fn close_sticky(app: &AppHandle) -> Result<(), anyhow::Error> {
    if let Some(window) = get_focused_window(app) {
        window.close()?;
        save_sticky(app, window.label(), None)?;
        Ok(())
    } else {
        bail!("No window currently focused!")
    }
}

pub fn sorted_windows(app: &AppHandle) -> Vec<WebviewWindow> {
    let mut positions: Vec<_> = app
        .webview_windows()
        .into_iter()
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

    sorted_windows[next_window_index].set_focus().context("Could not focus window")
}

pub fn fit_text(app: &AppHandle) -> Result<(), anyhow::Error> {
    app.webview_windows()
        .into_iter()
        .for_each(|(label, window)| {
            if window.is_focused().unwrap_or(false) {
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
            if window.is_focused().unwrap_or(false) {
                log::info!("emitting set color to window {}", label);
                let _ = window.emit_to(EventTarget::webview_window(label), "set_color", index);
            }
        });

    Ok(())
}

pub fn reset_note_positions(app: &AppHandle) -> anyhow::Result<()> {
    app
        .webview_windows()
        .into_iter()
        .map(|(_, window)| {
            window.set_position(PhysicalPosition { x: 0, y: 0 }).context("could not set note position")
        })
        .collect::<Result<(), anyhow::Error>>()
}

pub fn set_always_on_top(app: &AppHandle, always_on_top: bool) -> anyhow::Result<()> {
    if let Some(window) = get_focused_window(app) {
        window.set_always_on_top(always_on_top).context("Could not set window always on top")
    } else {
        bail!("No window currently focused!")
    }
}