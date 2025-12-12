use anyhow::Context;
use tauri::{AppHandle, Wry, menu::CheckMenuItem};

use crate::menu::MenuCommand;

pub struct MenuSettings {
    pub bring_to_front: CheckMenuItem<Wry>
}

impl MenuSettings {
    pub fn new(app: &AppHandle, bring_to_front: bool) -> anyhow::Result<Self> {
        Ok(Self {
            bring_to_front: CheckMenuItem::with_id(
                app, 
                MenuCommand::BringToFront, 
                "Bring all notes to front on focus", 
                true, 
                bring_to_front, 
                None::<String>
            )?
        })
    }

    pub fn bring_to_front(&self) -> anyhow::Result<bool> {
        self.bring_to_front.is_checked().context("Could not get checked menu item")
    }
}