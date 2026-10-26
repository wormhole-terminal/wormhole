//! `fallback/` is used only off macOS, which nothing builds yet, so it could drift from `macos/`
//! unseen. It is compiled on macOS too, and this pins each pair: the macOS function and the
//! portable one are put in the same `fn` type, so a signature that changes on one side only fails
//! to compile here. Methods are compared with `S` standing for each side's own type. Functions that
//! take `impl Fn` can't be a `fn` pointer: both are handed the same closure in code that never runs.

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

use gpui_kit::{App, Hsla, Window};
use wormhole_core::onboarding::intro::WindowMotion;
use wormhole_core::theme::tokens::Appearance;
use wormhole_core::view::motion::window_animation::Behaviour;
use wormhole_term::scrollbar::ScrollerStyle;

use super::font_face::Seg;
use super::fonts::Traits;
use super::intro::{Frame, Setup};
use super::notifications::{Buttons, FromCenter, Notice};
use super::vibrancy::GlassRect;
use super::{fallback, macos};

/// A glyph's outline, as both faces give it.
type Outline = Option<Vec<Seg>>;

/// Where the intro's page has its logo, shared with the overlay.
type LogoSpot = Rc<Cell<Option<[f32; 3]>>>;

/// The same `fn` type for both sides of a free function.
macro_rules! same {
    ($($m:ident :: $f:ident : $ty:ty;)*) => {
        $( let _: [$ty; 2] = [macos::$m::$f, fallback::$m::$f]; )*
    };
}

/// The same shape for both sides of a method, `S` being each side's own type.
macro_rules! same_method {
    ($($m:ident :: $t:ident :: $f:ident : $ty:ty;)*) => {
        $(
            {
                #[allow(dead_code)]
                type S = macos::$m::$t;
                let _: $ty = macos::$m::$t::$f;
            }
            {
                #[allow(dead_code)]
                type S = fallback::$m::$t;
                let _: $ty = fallback::$m::$t::$f;
            }
        )*
    };
}

/// Type-checks every pair; running it only takes function pointers.
pub(super) fn signatures_match() {
    same! {
        a11y::keyboard_navigation: fn() -> bool;
        a11y::post: fn(&str, bool);
        default_app::own_bundle: fn() -> Option<String>;
        default_app::runner_for: fn(&str) -> Option<String>;
        default_app::set_runner_for: fn(&str, &str) -> anyhow::Result<()>;
        apps::app_path: fn(&str) -> Option<std::path::PathBuf>;
        apps::bundle_of: fn(&Path) -> Option<String>;
        apps::icon_png: fn(&str, usize) -> Option<Vec<u8>>;
        apps::command: fn(&str, wormhole_core::platform::open_file::Launch) -> Option<std::process::Command>;
        apps::text_editor_command: fn(&Path) -> Option<std::process::Command>;
        dock::set_badge: fn(Option<&str>);
        dock::bounce: fn(&mut App);
        dock::is_active: fn() -> bool;
        dock::hide_icon: fn();
        fonts::classify: fn(&str) -> Option<Traits>;
        fonts::register: fn(&Path, bool) -> anyhow::Result<()>;
        hotkey::on_registering_thread: fn() -> bool;
        keys::option_sides: fn() -> (bool, bool);
        locale::preferred_languages: fn() -> Vec<String>;
        trash::move_to_trash: fn(&Path) -> anyhow::Result<()>;
        look::appearance: fn() -> Option<Appearance>;
        look::reduce_transparency: fn() -> bool;
        look::increase_contrast: fn() -> bool;
        look::reduce_motion: fn() -> Option<bool>;
        notifications::available: fn() -> bool;
        notifications::entitled_to_time_sensitive: fn() -> bool;
        notifications::script: fn(&Notice) -> Option<std::process::Command>;
        notifications::open_settings: fn(&App);
        scroller::style: fn(&App) -> ScrollerStyle;
        scroller::jump_to_spot: fn() -> bool;
        sheet::ask_with_checkbox: fn(&Window, &super::sheet::Question) -> Option<smol::channel::Receiver<Option<bool>>>;
        panel::hide_window_buttons: fn(&Window);
        panel::set_height_keeping_top: fn(&mut Window, f32, &mut App);
        panel::pointer_display_id: fn() -> Option<u64>;
        sound::beep: fn();
        vibrancy::set_material_appearance: fn(Appearance);
        window::lights_span: fn(&Window) -> Option<f32>;
        window::set_lights_shown: fn(&Window, bool);
        window::follow_theme: fn(&Window);
        window::set_top_left: fn(&Window, f32, f32);
        window::set_global_top_left: fn(&Window, f32, f32);
        window::make_ghost: fn(&Window);
        window::set_shadow: fn(&Window, bool);
        pointer::on_screen: fn() -> Option<super::pointer::Pointer>;
        pointer::screens: fn() -> Vec<wormhole_core::view::tear_off::Screen>;
        pointer::window_number_at: fn((f32, f32)) -> Option<i64>;
        window::number: fn(&Window) -> Option<i64>;
        window::is_minimised: fn(&Window) -> bool;
        window::allow_fullscreen: fn(&Window);
        window::deminiaturize: fn(&Window);
        window::hide: fn(&Window);
        window::show: fn(&Window);
        window::order_back: fn(&Window);
        window::keep_behind: fn(&Window);
        window::set_animation: fn(&Window, Behaviour);
        live_resize::set_increments: fn(&Window, Option<(f32, f32)>);
        live_resize::in_progress: fn(&Window) -> bool;
    }
    #[cfg(feature = "devctl")]
    same! {
        notice::force_material: fn(Option<super::notice::Material>);
    }
    #[cfg(feature = "devctl")]
    same_method! {
        notice::Chip::info: fn(&S) -> Option<super::notice::Info>;
    }
    #[cfg(feature = "devctl")]
    same! {
        a11y::wake_tree: fn(&Window);
        window::lights_opacity: fn(&Window) -> Option<(f32, bool)>;
        window::lights_state: fn(&Window) -> Option<String>;
        inspect::menu_bar: fn() -> Option<String>;
        inspect::menu: fn(&super::Menu, Option<usize>) -> anyhow::Result<String>;
        inspect::popover: fn(&super::Popover, &Path) -> anyhow::Result<String>;
        inspect::show_popover: fn(super::Popover, gpui_kit::Bounds<gpui_kit::Pixels>, &mut Window, &mut App) -> anyhow::Result<super::Handle>;
        status_item::save_icons: fn(&Path, &str, crate::menubar::StatusColors) -> anyhow::Result<Vec<std::path::PathBuf>>;
    }
    // Touch ID's implementations are left out of test builds, which use a stub of their own.
    #[cfg(not(test))]
    same! {
        auth::can_evaluate: fn(bool) -> Result<(), super::auth::AuthError>;
        auth::authenticate: fn(&str, super::auth::Callback);
    }
    #[cfg(feature = "devctl")]
    same! {
        intro::frames: fn(wormhole_core::onboarding::intro::Style, usize, [[f32; 4]; 3], [[f32; 3]; 3], &Path) -> anyhow::Result<String>;
        intro::dock_frames: fn(&Window, wormhole_core::onboarding::intro::voyage::Landing, [[f32; 4]; 3], &Path, &str) -> anyhow::Result<String>;
    }
    same_method! {
        notice::Chip::new: fn(&Window) -> Option<S>;
        notice::Chip::show: fn(&mut S, &super::notice::Content<'_>, f32, wormhole_core::view::motion::notice::Entrance);
        notice::Chip::swap: fn(&mut S, &super::notice::Content<'_>);
        notice::Chip::hide: fn(&mut S, wormhole_core::view::motion::notice::Entrance);
        notice::Chip::place: fn(&mut S, f32);
        intro::Overlay::new: fn(&Window, Setup, LogoSpot) -> anyhow::Result<Option<S>>;
        intro::Overlay::motion_at: fn(&S, f32) -> WindowMotion;
        intro::Overlay::frame: fn(&S, &Frame);
        font_face::Face::named: fn(&str, bool, bool) -> Option<S>;
        font_face::Face::for_char: fn(&S, char) -> Option<S>;
        font_face::Face::glyph_of: fn(&S, char) -> Option<u16>;
        font_face::Face::outline: fn(&S, u32, f32, (f32, f32), f32) -> Outline;
        vibrancy::BlurViews::install: fn(&Window, bool) -> Option<S>;
        vibrancy::BlurViews::set_backing: fn(&S, Option<Hsla>);
        vibrancy::BlurViews::set_enabled: fn(&S, bool);
        vibrancy::GlassViews::find: fn(&Window) -> Option<S>;
        vibrancy::GlassViews::place: fn(&mut S, &[GlassRect]);
        notifications::Center::open: fn(smol::channel::Sender<FromCenter>, Buttons) -> Option<S>;
        notifications::Center::post: fn(&S, &Notice);
        notifications::Center::remove: fn(&S, &str);
        notifications::Center::ask: fn(&S);
        notifications::Center::refresh_permission: fn(&S);
        status_item::MenuBar::install: fn(smol::channel::Sender<crate::menubar::MenuCommand>) -> Option<S>;
        status_item::MenuBar::update: fn(&mut S, &crate::menubar::MenuSnapshot);
        status_item::MenuBar::show_frame: fn(&mut S, Option<usize>);
    }
    // Never called: both sides must take the same kind of closure.
    let _ = || {
        let _ = macos::look::observe(|| {});
        let _ = fallback::look::observe(|| {});
        let _ = macos::scroller::observe(|| {});
        let _ = fallback::scroller::observe(|| {});
    };
}

#[cfg(test)]
mod tests {
    /// Compiling it is the check; this keeps it from being dead code in test builds.
    #[test]
    fn the_fallback_has_the_macos_signatures() {
        super::signatures_match();
    }
}
