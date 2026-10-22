//! GPUI's own implementations of what the platform draws natively elsewhere: for platforms that
//! have no native one yet, and for a window the native one can't reach.

#[cfg(not(target_os = "macos"))]
pub mod dock;
#[cfg(not(target_os = "macos"))]
pub mod font_face;
#[cfg(not(target_os = "macos"))]
pub mod look;
pub mod menu;
pub mod popover;
#[cfg(not(target_os = "macos"))]
pub mod scroller;
#[cfg(not(target_os = "macos"))]
pub mod vibrancy;
#[cfg(not(target_os = "macos"))]
pub mod window;

pub use menu::layer;
pub use popover::layer as popover_layer;
