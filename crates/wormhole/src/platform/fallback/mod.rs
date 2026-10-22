//! GPUI's own implementations of what the platform draws natively elsewhere: for platforms that
//! have no native one yet, and for a window the native one can't reach.

#[cfg(not(target_os = "macos"))]
pub mod a11y;
#[cfg(all(not(target_os = "macos"), not(test)))]
pub mod auth;
#[cfg(not(target_os = "macos"))]
pub mod dock;
#[cfg(not(target_os = "macos"))]
pub mod font_face;
#[cfg(not(target_os = "macos"))]
pub mod fonts;
#[cfg(not(target_os = "macos"))]
pub mod hotkey;
#[cfg(not(target_os = "macos"))]
pub mod keys;
#[cfg(not(target_os = "macos"))]
pub mod look;
#[cfg(not(target_os = "macos"))]
pub mod scroller;
#[cfg(not(target_os = "macos"))]
pub mod sound;
#[cfg(not(target_os = "macos"))]
pub mod status_item;
#[cfg(not(target_os = "macos"))]
pub mod vibrancy;
#[cfg(not(target_os = "macos"))]
pub mod window;
