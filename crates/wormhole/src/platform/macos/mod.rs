//! Everything AppKit that native features need: the only place of this module tree with `objc2`
//! and `unsafe`.

pub mod a11y;
pub mod appkit;
#[cfg(not(test))]
pub mod auth;
pub mod colour;
pub mod dock;
pub mod font_face;
pub mod fonts;
pub mod hotkey;
pub mod keys;
pub mod look;
pub mod menu;
pub mod popover;
mod rows;
pub mod scroller;
pub mod sound;
pub mod status_item;
mod swatches;
pub mod vibrancy;
pub mod window;
