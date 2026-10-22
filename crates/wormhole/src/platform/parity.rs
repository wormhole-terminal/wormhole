//! `fallback/` is used only off macOS, which nothing builds yet, so it could drift from `macos/`
//! unseen. It is compiled on macOS too, and this pins each pair: the macOS function and the
//! portable one are put in the same `fn` type, so a signature that changes on one side only fails
//! to compile here. Methods are compared with `S` standing for each side's own type. Functions that
//! take `impl Fn` can't be a `fn` pointer: both are handed the same closure in code that never runs.

use std::path::Path;

use gpui_kit::{App, Hsla, Window};
use wormhole_core::terminal::scrollbar::ScrollerStyle;
use wormhole_core::tokens::Appearance;

use super::font_face::Seg;
use super::fonts::Traits;
use super::vibrancy::GlassRect;
use super::{fallback, macos};

/// A glyph's outline, as both faces give it.
type Outline = Option<Vec<Seg>>;

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
        dock::set_badge: fn(Option<&str>);
        dock::bounce: fn(&mut App);
        dock::is_active: fn() -> bool;
        dock::hide_icon: fn();
        fonts::classify: fn(&str) -> Option<Traits>;
        fonts::register: fn(&Path, bool) -> anyhow::Result<()>;
        hotkey::on_registering_thread: fn() -> bool;
        keys::option_sides: fn() -> (bool, bool);
        look::appearance: fn() -> Option<Appearance>;
        look::reduce_transparency: fn() -> bool;
        look::increase_contrast: fn() -> bool;
        look::reduce_motion: fn() -> Option<bool>;
        scroller::style: fn(&App) -> ScrollerStyle;
        scroller::jump_to_spot: fn() -> bool;
        sound::beep: fn();
        vibrancy::glass_available: fn() -> bool;
        vibrancy::set_material_appearance: fn(Appearance);
        window::lights_span: fn(&Window) -> Option<f32>;
        window::set_lights_shown: fn(&Window, bool);
        window::set_top_left: fn(&Window, &App, f32, f32);
        window::is_minimised: fn(&Window) -> bool;
        window::allow_fullscreen: fn(&Window);
        window::deminiaturize: fn(&Window);
        window::hide: fn(&Window);
        window::show: fn(&Window);
        window::order_back: fn(&Window);
        window::keep_behind: fn(&Window);
    }
    #[cfg(feature = "devctl")]
    same! {
        a11y::wake_tree: fn(&Window);
        window::lights_opacity: fn(&Window) -> Option<(f32, bool)>;
        status_item::save_icons: fn(&Path, &str, crate::menubar::StatusColors) -> anyhow::Result<Vec<std::path::PathBuf>>;
    }
    // Touch ID's implementations are left out of test builds, which use a stub of their own.
    #[cfg(not(test))]
    same! {
        auth::can_evaluate: fn(bool) -> Result<(), super::auth::AuthError>;
        auth::authenticate: fn(&str, super::auth::Callback);
    }
    same_method! {
        font_face::Face::named: fn(&str, bool, bool) -> Option<S>;
        font_face::Face::for_char: fn(&S, char) -> Option<S>;
        font_face::Face::glyph_of: fn(&S, char) -> Option<u16>;
        font_face::Face::outline: fn(&S, u32, f32, (f32, f32), f32) -> Outline;
        vibrancy::BlurViews::install: fn(&Window, bool) -> Option<S>;
        vibrancy::BlurViews::set_backing: fn(&S, Option<Hsla>);
        vibrancy::BlurViews::set_enabled: fn(&S, bool);
        vibrancy::GlassViews::find: fn(&Window) -> Option<S>;
        vibrancy::GlassViews::place: fn(&mut S, &[GlassRect]);
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
