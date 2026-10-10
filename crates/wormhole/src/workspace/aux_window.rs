//! What the secondary windows (settings, Theme Studio, About, Licenses) share: window
//! options, surfaces that are either translucent like the main window's (window opacity +
//! native blur) or fully opaque, and for the small ones a window that exists at most once and
//! closes with Esc or ⌘W. Under Reduce Transparency the settings in force say an opacity of 1,
//! so a window asked to be translucent gets the opaque surfaces and no blur too.

use wormhole_core::tokens::{Color, OPAQUE};

use super::app::app_mut;
use super::*;
use crate::settings::Settings;
use crate::theme_bridge::hsla;
use crate::workspace::chrome::TitleDrag as _;

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
pub(crate) fn surfaces(cx: &App, translucent: bool) -> Surfaces {
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

/// Options for a secondary window: transparent title bar with the traffic
/// lights where the main window has them, and a transparent background so
/// translucency can be switched on and off without reopening it.
pub(crate) fn window_options(
    title: &'static str,
    initial: Size<Pixels>,
    min: Size<Pixels>,
    cx: &App,
) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, initial, cx))),
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            appears_transparent: true,
            traffic_light_position: Some(super::chrome::traffic_light_position()),
        }),
        window_background: WindowBackgroundAppearance::Transparent,
        window_min_size: Some(min),
        // The title rows drag the window themselves (`chrome::TitleDrag`).
        app_owns_titlebar_drag: true,
        ..Default::default()
    }
}

/// A secondary window that exists at most once: its handle, and its view (for devctl).
pub(crate) type Slot = Option<(AnyWindowHandle, AnyWeakEntity)>;

/// Bring the window in `slot` to the front, or open a new one there.
pub(crate) fn open_single<V: Render + 'static>(
    slot: fn(&mut AppState) -> &mut Slot,
    options: WindowOptions,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V> + 'static,
    cx: &mut App,
) {
    if let Some((handle, _)) = slot(app_mut(cx)).clone()
        && handle.update(cx, |_, window, cx| crate::background::activate_window(window, cx)).is_ok()
    {
        return;
    }
    // Deferred: the new window renders right away, and this may run inside another window's
    // update.
    cx.defer(move |cx| match crate::background::open_window(options, cx, build) {
        Ok((handle, view)) => *slot(app_mut(cx)) = Some((handle, view.downgrade().into())),
        Err(err) => tracing::error!("opening a window failed: {err:#}"),
    });
}

/// Esc and ⌘W close a small window, as they do the settings window.
pub(crate) fn close_on_esc_or_cmd_w(event: &KeyDownEvent, window: &mut Window) {
    let keys = &event.keystroke;
    if keys.key == "escape" || (keys.modifiers.platform && keys.key == "w") {
        window.remove_window();
    }
}

/// A strip along the top to drag the window by (its title bar is transparent).
pub(crate) fn drag_strip() -> Div {
    div().absolute().top_0().left_0().right_0().h(px(style::TITLE_ROW_H)).title_drag()
}

/// Reduce Transparency in a headless GPUI (needs `devctl`'s test support).
#[cfg(all(test, feature = "devctl"))]
mod tests {
    use gpui_kit::TestAppContext;

    use super::{surfaces, wants_blur};
    use crate::settings::{SettingsStore, SystemLook, settings};

    #[gpui_kit::test]
    fn reduce_transparency_gives_translucent_windows_opaque_surfaces(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_global(SettingsStore::for_test());
            let alphas = |cx: &gpui_kit::App| {
                let s = surfaces(cx, true);
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
