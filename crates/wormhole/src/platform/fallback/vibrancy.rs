//! `platform::vibrancy` without a blur or glass of the system's: a window draws its own opaque
//! surfaces, so the blur is a switch that changes nothing and there is no glass.

use gpui_kit::{Hsla, Window};
use wormhole_core::theme::tokens::Appearance;

use crate::platform::vibrancy::GlassRect;

pub struct BlurViews;

impl BlurViews {
    pub fn install(_: &Window, _: bool) -> Option<Self> {
        Some(Self)
    }

    pub fn set_backing(&self, _: Option<Hsla>) {}

    pub fn set_enabled(&self, _: bool) {}
}

pub fn glass_available() -> bool {
    false
}

pub struct GlassViews;

impl GlassViews {
    pub fn find(_: &Window) -> Option<Self> {
        None
    }

    pub fn place(&mut self, _: &[GlassRect]) {}
}

pub fn set_material_appearance(_: Appearance) {}
