use anyhow::Context;
use tauri::menu::{
    Menu, MenuBuilder, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu, SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Wry};
use tauri_plugin_log::log;

use crate::windows::{close_sticky, create_sticky, cycle_focus, fit_text, snap_window, Direction};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
enum MenuCommand {
    NewNote,
    CloseNote,
    FitText,
    NextNote,
    PrevNote,
    Snap(Direction),
    PartialSnap(Direction),
}

impl Into<MenuId> for MenuCommand {
    fn into(self) -> MenuId {
        MenuId(serde_json::to_string(&self).expect("Could not serialize MenuCommand enum"))
    }
}

impl TryFrom<MenuId> for MenuCommand {
    type Error = anyhow::Error;
    fn try_from(value: MenuId) -> Result<Self, Self::Error> {
        serde_json::from_str(&value.0).context(format!(
            "Could not deserialize {:?} into MenuCommand",
            value
        ))
    }
}

fn create_window_submenu(app: &AppHandle) -> Result<Submenu<Wry>, anyhow::Error> {
    let menu = SubmenuBuilder::new(app, "About")
        .items(&[
            &PredefinedMenuItem::quit(app, None)?,
            &MenuItem::with_id(
                app,
                MenuCommand::CloseNote,
                "Close Note",
                true,
                Some("Cmd+W"),
            )?,
            &MenuItem::with_id(app, MenuCommand::NewNote, "New Note", true, Some("Cmd+N"))?,
            &MenuItem::with_id(
                app,
                MenuCommand::NextNote,
                "Focus Next Note",
                true,
                Some("Cmd+/"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::PrevNote,
                "Focus Previous Note",
                true,
                Some("Cmd+Alt+/"),
            )?,
        ])
        .build()?;

    Ok(menu)
}

fn create_snap_submenu(app: &AppHandle) -> Result<Submenu<Wry>, anyhow::Error> {
    let menu = SubmenuBuilder::new(app, "Snap")
        .items(&[
            &MenuItem::with_id(
                app,
                MenuCommand::Snap(Direction::Up),
                "Up",
                true,
                Some("Cmd+Alt+Up"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::Snap(Direction::Down),
                "Down",
                true,
                Some("Cmd+Alt+Down"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::Snap(Direction::Left),
                "Left",
                true,
                Some("Cmd+Alt+Left"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::Snap(Direction::Right),
                "Right",
                true,
                Some("Cmd+Alt+Right"),
            )?,
        ])
        .build()?;

    Ok(menu)
}

fn create_partial_snap_submenu(app: &AppHandle) -> Result<Submenu<Wry>, anyhow::Error> {
    let menu = SubmenuBuilder::new(app, "Partial Snap")
        .items(&[
            &MenuItem::with_id(
                app,
                MenuCommand::PartialSnap(Direction::Up),
                "Up",
                true,
                Some("Cmd+Alt+Shift+Up"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::PartialSnap(Direction::Down),
                "Down",
                true,
                Some("Cmd+Alt+Shift+Down"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::PartialSnap(Direction::Left),
                "Left",
                true,
                Some("Cmd+Alt+Shift+Left"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::PartialSnap(Direction::Right),
                "Right",
                true,
                Some("Cmd+Alt+Shift+Right"),
            )?,
        ])
        .build()?;

    Ok(menu)
}

fn create_edit_submenu(app: &AppHandle) -> Result<Submenu<Wry>, anyhow::Error> {
    let menu = SubmenuBuilder::new(app, "Edit")
        .items(&[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &MenuItem::with_id(
                app,
                MenuCommand::FitText,
                "Resize Note to Text",
                true,
                Some("Cmd+F"),
            )?,
        ])
        .build()?;

    Ok(menu)
}

pub fn create_menu(app: &AppHandle) -> Result<Menu<Wry>, anyhow::Error> {
    let menu = MenuBuilder::new(app)
        .items(&[
            &create_window_submenu(app)?,
            &create_edit_submenu(app)?,
            &create_snap_submenu(app)?,
            &create_partial_snap_submenu(app)?,
        ])
        .build()?;

    Ok(menu)
}

pub fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match MenuCommand::try_from(event.id) {
        Ok(command) => {
            if let Err(e) = match command {
                MenuCommand::NewNote => create_sticky(app, None).map(|_| ()),
                MenuCommand::Snap(direction) => snap_window(app, direction, false),
                MenuCommand::PartialSnap(direction) => snap_window(app, direction, true),
                MenuCommand::CloseNote => close_sticky(app),
                MenuCommand::NextNote => cycle_focus(app, false),
                MenuCommand::PrevNote => cycle_focus(app, true),
                MenuCommand::FitText => fit_text(app),
                // _ => Err(anyhow!("unimplemented command: {:?}", command)),
            } {
                log::error!("Error executing command: {:?} : {:#}", command, e)
            }
        }
        Err(e) => {
            log::warn!("{:#}", e)
        }
    };

    let _ = app.emit("save_request", {});
}
