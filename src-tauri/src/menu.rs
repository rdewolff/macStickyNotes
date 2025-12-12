use anyhow::{Context};
use tauri::menu::{
    Menu, MenuBuilder, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu, SubmenuBuilder
};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_log::log;

use crate::save_load::save_settings;
use crate::settings::MenuSettings;
use crate::windows::{close_sticky, create_sticky, cycle_focus, fit_text, reset_note_positions, set_color, snap_window, Direction};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
pub enum MenuCommand {
    NewNote,
    CloseNote,
    ResetPositions,
    FitText,
    NextNote,
    PrevNote,
    Color(u8),
    Snap(Direction),
    PartialSnap(Direction),
    BringToFront,
    AutoStart,
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
    let settings = app.state::<MenuSettings>();

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
            &MenuItem::with_id(app, MenuCommand::ResetPositions, "Reset Note Positions", true, None::<&str>)?
        ])
        .separator()
        .items(&[
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
        .separator()
        .items(&[
            &settings.bring_to_front,
            &settings.autostart,
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
        ])
        .separator()
        .items(&[
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
        ])
        .separator()
        .item(&MenuItem::with_id(
                app,
                MenuCommand::FitText,
                "Resize Note to Text",
                true,
                Some("Cmd+F"),
            )?,)
        .build()?;

    Ok(menu)
}

fn create_color_menu(app: &AppHandle) -> Result<Submenu<Wry>, anyhow::Error> {
    let menu = SubmenuBuilder::new(app, "Color")
        .items(&[
            &MenuItem::with_id(
                app,
                MenuCommand::Color(0),
                "Color 1",
                true,
                Some("Cmd+1"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::Color(1),
                "Color 2",
                true,
                Some("Cmd+2"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::Color(2),
                "Color 3",
                true,
                Some("Cmd+3"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::Color(3),
                "Color 4",
                true,
                Some("Cmd+4"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::Color(4),
                "Color 5",
                true,
                Some("Cmd+5"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::Color(5),
                "Color 6",
                true,
                Some("Cmd+6"),
            )?,
            &MenuItem::with_id(
                app,
                MenuCommand::Color(6),
                "Color 7",
                true,
                Some("Cmd+7"),
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
            &create_color_menu(app)?,
        ])
        .build()?;

    Ok(menu)
}

pub fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match MenuCommand::try_from(event.id) {
        Ok(command) => {
            if let Err(e) = match command {
                MenuCommand::NewNote => create_sticky(app, None).map(|_| ()),
                MenuCommand::ResetPositions => reset_note_positions(app),
                MenuCommand::Snap(direction) => snap_window(app, direction, false),
                MenuCommand::PartialSnap(direction) => snap_window(app, direction, true),
                MenuCommand::CloseNote => close_sticky(app),
                MenuCommand::NextNote => cycle_focus(app, false),
                MenuCommand::PrevNote => cycle_focus(app, true),
                MenuCommand::FitText => fit_text(app),
                MenuCommand::Color(index) => set_color(app, index), 
                MenuCommand::BringToFront => save_settings(app),
                MenuCommand::AutoStart => save_settings(app),
                // _ => Err(anyhow::anyhow!("unimplemented command: {:?}", command)),
            } {
                log::error!("Error executing command: {:?} : {:#}", command, e);
            };
            if let 
                MenuCommand::NewNote |
                MenuCommand::CloseNote |
                MenuCommand::Color(_) 
            = command {
                _ = app.emit("save_request", {});
            };
        }
        Err(e) => {
            log::warn!("{:#}", e)
        }
    };
}
