//! `platform::notifications` without notifications of the system's own: no centre and no script,
//! so the app posts through GPUI's portable notifications.

use std::process::Command;

use gpui_kit::App;
use smol::channel::Sender;

use crate::platform::notifications::{Buttons, FromCenter, Notice};

pub struct Center;

impl Center {
    pub fn open(_tx: Sender<FromCenter>, _buttons: Buttons) -> Option<Self> {
        None
    }

    pub fn post(&self, _notice: &Notice) {}

    pub fn remove(&self, _id: &str) {}

    pub fn ask(&self) {}

    pub fn refresh_permission(&self) {}
}

pub fn available() -> bool {
    false
}

pub fn entitled_to_time_sensitive() -> bool {
    false
}

pub fn script(_notice: &Notice) -> Option<Command> {
    None
}

pub fn open_settings(_cx: &App) {}
