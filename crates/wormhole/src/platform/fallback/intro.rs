//! `platform::intro` without an overlay: GPUI has no window around another or custom shaders, so
//! the intro doesn't play and devctl can't draw it.

use std::cell::Cell;
use std::rc::Rc;

use anyhow::Result;
use gpui_kit::Window;
use wormhole_core::onboarding::intro::WindowMotion;

use crate::platform::intro::{Frame, Setup};

pub struct Overlay;

impl Overlay {
    pub fn new(
        _window: &Window,
        _setup: Setup,
        _landing: Rc<Cell<Option<[f32; 3]>>>,
    ) -> Result<Option<Self>> {
        Ok(None)
    }

    pub fn motion_at(&self, _t: f32) -> WindowMotion {
        WindowMotion::AT_REST
    }

    pub fn frame(&self, _frame: &Frame) {}
}

#[cfg(feature = "devctl")]
pub fn frames(
    _: wormhole_core::onboarding::intro::Style,
    _: usize,
    _: [[f32; 4]; 3],
    _: [[f32; 3]; 3],
    _: &std::path::Path,
) -> Result<String> {
    anyhow::bail!("the intro draws with Metal: macOS only")
}

#[cfg(feature = "devctl")]
pub fn dock_frames(
    _: &Window,
    _: wormhole_core::onboarding::intro::voyage::Landing,
    _: [[f32; 4]; 3],
    _: &std::path::Path,
    _: &str,
) -> Result<String> {
    anyhow::bail!("the intro draws with Metal: macOS only")
}
