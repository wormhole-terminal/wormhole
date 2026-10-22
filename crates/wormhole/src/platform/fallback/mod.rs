//! GPUI's own implementations of what the platform draws natively elsewhere: for platforms that
//! have no native one yet, and for a window the native one can't reach.

#[cfg(not(target_os = "macos"))]
pub mod font_face;
pub mod menu;
pub mod popover;

pub use menu::layer;
pub use popover::layer as popover_layer;
