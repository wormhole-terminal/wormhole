//! What a window the app draws itself (settings, the Theme Studio, About, the trust window and the intro
//! window) shares: surfaces that are either translucent like the main window's (window opacity + native
//! blur) or fully opaque, and the strip to drag the window by. Under Reduce Transparency the settings in
//! force say an opacity of 1, so a window asked to be translucent gets the opaque surfaces and no blur
//! too.

use gpui_kit::*;
use wormhole_core::theme::tokens::{Color, OPAQUE};

use super::{TITLE_ROW_H, TitleDrag as _};
use crate::settings::{Settings, SettingsStore};
use crate::theme_bridge::hsla;

/// Surface colors for a secondary window.
#[derive(Clone, Copy)]
pub(crate) struct Surfaces {
    pub background: Hsla,
    pub panel: Hsla,
    pub panel_right: Hsla,
}

/// The window's surfaces. Translucent ones are the main window's (tokens at
/// the window opacity); opaque ones are composited over an opaque window
/// color, because a transparent window shows black through anything with alpha.
pub(crate) fn window_surfaces(cx: &App, translucent: bool) -> Surfaces {
    let store = cx.global::<SettingsStore>();
    if translucent && store.settings.opacity < OPAQUE {
        let ui = store.ui;
        return Surfaces { background: ui.background, panel: ui.panel, panel_right: ui.panel_right };
    }
    let tokens = &store.tokens;
    let magenta = Color::rgba(1.0, 0.0, 1.0, 1.0);
    let window = tokens.get("surface.window").unwrap_or(magenta).with_alpha(1.0);
    let opaque = |name: &str| hsla(tokens.get(name).unwrap_or(magenta).over(window));
    Surfaces {
        background: hsla(window),
        panel: opaque("sidebar.background"),
        panel_right: opaque("surface.panel"),
    }
}

/// Whether a window with this translucency setting should blur what's behind it.
pub(crate) fn wants_blur(settings: &Settings, translucent: bool) -> bool {
    translucent && settings.blur && settings.opacity < OPAQUE
}

/// A strip along the top to drag the window by (its title bar is transparent).
#[track_caller]
pub(crate) fn drag_strip() -> Div {
    div().absolute().top_0().left_0().right_0().h(px(TITLE_ROW_H)).title_drag()
}

/// Reduce Transparency in a headless GPUI (needs `devctl`'s test support).
#[cfg(all(test, feature = "devctl"))]
mod tests {
    use gpui_kit::TestAppContext;

    use super::{wants_blur, window_surfaces};
    use crate::settings::{SettingsStore, SystemLook, settings};

    #[gpui_kit::test]
    fn reduce_transparency_gives_translucent_windows_opaque_surfaces(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_global(SettingsStore::for_test());
            let alphas = |cx: &gpui_kit::App| {
                let s = window_surfaces(cx, true);
                [s.background.a, s.panel.a, s.panel_right.a]
            };
            assert!(alphas(cx).iter().all(|a| *a < 1.0), "the default 80 % lets the desktop through");
            assert!(wants_blur(settings(cx), true));

            let reduced = SystemLook { reduce_transparency: true, ..SystemLook::default() };
            assert!(SettingsStore::set_system(cx, reduced));
            assert_eq!(alphas(cx), [1.0; 3]);
            assert!(!wants_blur(settings(cx), true), "the effect view is hidden");

            assert!(SettingsStore::set_system(cx, SystemLook::default()));
            assert!(alphas(cx).iter().all(|a| *a < 1.0));
        });
    }
}
