use std::collections::HashMap;
use std::sync::Mutex;

use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionaryRef;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::display::{
    kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
    CGWindowListCopyWindowInfo,
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_log::log;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AnchorInfo {
    pub target_window_id: u32,
    pub offset_x: f64,
    pub offset_y: f64,
    pub target_app_name: String,
}

#[derive(Debug, Default)]
pub struct AnchorState {
    pub anchors: Mutex<HashMap<String, AnchorInfo>>,
    pub polling_active: Mutex<bool>,
}

#[derive(Debug, Clone)]
struct ExternalWindow {
    id: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    owner_name: String,
}

fn get_external_windows(own_pid: u32) -> Vec<ExternalWindow> {
    let mut windows = Vec::new();

    unsafe {
        let window_list = CGWindowListCopyWindowInfo(
            kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
            kCGNullWindowID,
        );

        if window_list.is_null() {
            return windows;
        }

        let cf_array = core_foundation::array::CFArray::<CFType>::wrap_under_get_rule(
            window_list as core_foundation::array::CFArrayRef,
        );
        let count = cf_array.len();

        for i in 0..count {
            let item = match cf_array.get(i) {
                Some(item) => item,
                None => continue,
            };
            let dict_ref = item.as_CFTypeRef() as CFDictionaryRef;

            let layer = get_dict_number(dict_ref, "kCGWindowLayer");
            if layer != Some(0) {
                continue;
            }

            let pid = get_dict_number(dict_ref, "kCGWindowOwnerPID");
            if pid == Some(own_pid as i64) {
                continue;
            }

            let window_id = match get_dict_number(dict_ref, "kCGWindowNumber") {
                Some(id) => id as u32,
                None => continue,
            };

            let owner_name = get_dict_string(dict_ref, "kCGWindowOwnerName")
                .unwrap_or_default();

            let bounds = match get_dict_bounds(dict_ref) {
                Some(b) => b,
                None => continue,
            };

            if bounds.2 < 50.0 || bounds.3 < 50.0 {
                continue;
            }

            windows.push(ExternalWindow {
                id: window_id,
                x: bounds.0,
                y: bounds.1,
                width: bounds.2,
                height: bounds.3,
                owner_name,
            });
        }
    }

    windows
}

unsafe fn get_dict_number(dict: CFDictionaryRef, key: &str) -> Option<i64> {
    let cf_key = CFString::new(key);
    let mut value: *const core::ffi::c_void = std::ptr::null();
    if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
        dict,
        cf_key.as_concrete_TypeRef() as *const core::ffi::c_void,
        &mut value,
    ) != 0
    {
        let cf_num = CFNumber::wrap_under_get_rule(value as core_foundation::number::CFNumberRef);
        cf_num.to_i64()
    } else {
        None
    }
}

unsafe fn get_dict_string(dict: CFDictionaryRef, key: &str) -> Option<String> {
    let cf_key = CFString::new(key);
    let mut value: *const core::ffi::c_void = std::ptr::null();
    if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
        dict,
        cf_key.as_concrete_TypeRef() as *const core::ffi::c_void,
        &mut value,
    ) != 0
    {
        let cf_str =
            CFString::wrap_under_get_rule(value as core_foundation::string::CFStringRef);
        Some(cf_str.to_string())
    } else {
        None
    }
}

unsafe fn get_dict_bounds(dict: CFDictionaryRef) -> Option<(f64, f64, f64, f64)> {
    let cf_key = CFString::new("kCGWindowBounds");
    let mut value: *const core::ffi::c_void = std::ptr::null();
    if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
        dict,
        cf_key.as_concrete_TypeRef() as *const core::ffi::c_void,
        &mut value,
    ) != 0
    {
        let bounds_dict = value as CFDictionaryRef;
        let x = get_dict_number(bounds_dict, "X").unwrap_or(0) as f64;
        let y = get_dict_number(bounds_dict, "Y").unwrap_or(0) as f64;
        let w = get_dict_number(bounds_dict, "Width").unwrap_or(0) as f64;
        let h = get_dict_number(bounds_dict, "Height").unwrap_or(0) as f64;
        Some((x, y, w, h))
    } else {
        None
    }
}

fn get_own_pid() -> u32 {
    std::process::id()
}

fn find_nearest_window(
    sticky_x: f64,
    sticky_y: f64,
    sticky_w: f64,
    sticky_h: f64,
    external_windows: &[ExternalWindow],
) -> Option<&ExternalWindow> {
    let sticky_cx = sticky_x + sticky_w / 2.0;
    let sticky_cy = sticky_y + sticky_h / 2.0;

    external_windows
        .iter()
        .min_by(|a, b| {
            let a_cx = a.x + a.width / 2.0;
            let a_cy = a.y + a.height / 2.0;
            let dist_a = (sticky_cx - a_cx).powi(2) + (sticky_cy - a_cy).powi(2);

            let b_cx = b.x + b.width / 2.0;
            let b_cy = b.y + b.height / 2.0;
            let dist_b = (sticky_cx - b_cx).powi(2) + (sticky_cy - b_cy).powi(2);

            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn find_window_by_id(id: u32, windows: &[ExternalWindow]) -> Option<&ExternalWindow> {
    windows.iter().find(|w| w.id == id)
}

pub fn anchor_to_nearest(app: &AppHandle, window: &WebviewWindow) -> Result<String, anyhow::Error> {
    let own_pid = get_own_pid();
    let external = get_external_windows(own_pid);

    if external.is_empty() {
        anyhow::bail!("No external windows found to anchor to");
    }

    let scale_factor = window.scale_factor()?;
    let pos = window.outer_position()?.to_logical::<f64>(scale_factor);
    let size = window.outer_size()?.to_logical::<f64>(scale_factor);

    let nearest = find_nearest_window(pos.x, pos.y, size.width, size.height, &external)
        .ok_or_else(|| anyhow::anyhow!("No nearest window found"))?;

    let offset_x = pos.x - nearest.x;
    let offset_y = pos.y - nearest.y;

    let info = AnchorInfo {
        target_window_id: nearest.id,
        offset_x,
        offset_y,
        target_app_name: nearest.owner_name.clone(),
    };

    let label = window.label().to_string();
    let target_name = info.target_app_name.clone();

    let state = app.state::<AnchorState>();
    state.anchors.lock().unwrap().insert(label.clone(), info);

    log::info!(
        "Anchored {} to window {} ({})",
        label,
        nearest.id,
        target_name
    );

    start_polling_if_needed(app);

    Ok(target_name)
}

pub fn unanchor(app: &AppHandle, window: &WebviewWindow) -> Result<(), anyhow::Error> {
    let label = window.label().to_string();
    let state = app.state::<AnchorState>();
    state.anchors.lock().unwrap().remove(&label);

    log::info!("Unanchored {}", label);
    Ok(())
}

fn start_polling_if_needed(app: &AppHandle) {
    let state = app.state::<AnchorState>();
    let mut polling = state.polling_active.lock().unwrap();
    if *polling {
        return;
    }
    *polling = true;
    drop(polling);

    let app_handle = app.clone();

    std::thread::spawn(move || {
        log::info!("Anchor polling loop started");
        let own_pid = get_own_pid();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(150));

            let state = app_handle.state::<AnchorState>();
            let anchors = state.anchors.lock().unwrap().clone();

            if anchors.is_empty() {
                let mut polling = state.polling_active.lock().unwrap();
                *polling = false;
                log::info!("Anchor polling loop stopped (no anchors)");
                break;
            }

            let external = get_external_windows(own_pid);

            let mut to_remove: Vec<String> = Vec::new();

            for (label, anchor_info) in &anchors {
                if let Some(target) = find_window_by_id(anchor_info.target_window_id, &external) {
                    let new_x = target.x + anchor_info.offset_x;
                    let new_y = target.y + anchor_info.offset_y;

                    if let Some(window) = app_handle.webview_windows().get(label) {
                        let scale_factor = window.scale_factor().unwrap_or(1.0);
                        let current_pos = window
                            .outer_position()
                            .map(|p| p.to_logical::<f64>(scale_factor));

                        if let Ok(current) = current_pos {
                            let dx = (current.x - new_x).abs();
                            let dy = (current.y - new_y).abs();
                            if dx > 1.0 || dy > 1.0 {
                                let physical_pos = tauri::LogicalPosition::new(new_x, new_y);
                                let _ = window.set_position(physical_pos);
                            }
                        }
                    } else {
                        to_remove.push(label.clone());
                    }
                } else {
                    to_remove.push(label.clone());
                    let _ = app_handle.emit_to(
                        tauri::EventTarget::webview_window(label.clone()),
                        "anchor_lost",
                        (),
                    );
                    log::info!("Target window closed for anchor {}", label);
                }
            }

            if !to_remove.is_empty() {
                let state = app_handle.state::<AnchorState>();
                let mut anchors = state.anchors.lock().unwrap();
                for label in to_remove {
                    anchors.remove(&label);
                }
            }
        }
    });
}
