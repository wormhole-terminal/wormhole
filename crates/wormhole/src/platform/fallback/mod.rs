//! The portable side of each platform concern, for a system that isn't macOS: GPUI's own call
//! where GPUI has one (the Dock's bounce is `request_attention`, the scroller style
//! `should_auto_hide_scrollbars`), and "not available" where it hasn't. Used only off macOS,
//! which isn't a target yet; compiled everywhere, and `parity` keeps it to `macos/`'s functions.

pub mod a11y;
pub mod apps;
#[cfg(not(test))]
pub mod auth;
pub mod default_app;
pub mod dock;
pub mod font_face;
pub mod fonts;
pub mod hotkey;
#[cfg(feature = "devctl")]
pub mod inspect;
pub mod intro;
pub mod keys;
pub mod locale;
pub mod look;
pub mod notifications;
pub mod panel;
pub mod pointer;
pub mod scroller;
pub mod sheet;
pub mod sound;
pub mod status_item;
pub mod trash;
pub mod vibrancy;
pub mod window;
