//! Everything AppKit that native features need: the only place of this module tree with `objc2`
//! and `unsafe`.

pub mod a11y;
mod appearance;
pub mod appkit;
pub mod apps;
#[cfg(not(test))]
pub mod auth;
pub mod buttons;
pub mod colour;
pub mod controls;
pub mod default_app;
pub mod dock;
pub mod font_face;
pub mod fonts;
pub mod hotkey;
pub mod image;
#[cfg(feature = "devctl")]
pub mod inspect;
pub mod intro;
pub mod keys;
pub mod live_resize;
pub mod locale;
pub mod look;
pub mod menu;
mod native_controls;
pub mod notice;
mod notice_click;
mod notice_layout;
mod notice_motion;
pub mod notifications;
pub mod panel;
pub mod picker;
mod picker_backdrop;
mod picker_crumbs;
mod picker_slot;
mod picker_views;
pub mod pointer;
pub mod popover;
mod rows;
pub mod scroller;
pub mod segments;
pub mod sheet;
pub mod sliders;
pub mod sound;
pub mod status_item;
pub mod steppers;
mod swatches;
pub mod trash;
pub mod vibrancy;
pub mod window;
