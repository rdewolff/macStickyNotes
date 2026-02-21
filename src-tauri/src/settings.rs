use anyhow::Context;
use tauri::{menu::CheckMenuItem, AppHandle, Wry};

use crate::menu::MenuCommand;

pub struct MenuSettings {
    pub bring_to_front: CheckMenuItem<Wry>,
    pub autostart: CheckMenuItem<Wry>,
}

impl MenuSettings {
    pub fn new(app: &AppHandle, bring_to_front: bool, autostart: bool) -> anyhow::Result<Self> {
        Ok(Self {
            bring_to_front: CheckMenuItem::with_id(
                app,
                MenuCommand::BringToFront,
                "Bring all notes to front on focus",
                true,
                bring_to_front,
                None::<String>,
            )?,
            autostart: CheckMenuItem::with_id(
                app,
                MenuCommand::AutoStart,
                "Launch app on startup",
                true,
                autostart,
                None::<String>,
            )?,
        })
    }

    fn get_checked_status(item: &CheckMenuItem<Wry>) -> anyhow::Result<bool> {
        item.is_checked().context("Could not get checked menu item")
    }

    pub fn bring_to_front(&self) -> anyhow::Result<bool> {
        Self::get_checked_status(&self.bring_to_front)
    }

    pub fn autostart(&self) -> anyhow::Result<bool> {
        Self::get_checked_status(&self.autostart)
    }
}
