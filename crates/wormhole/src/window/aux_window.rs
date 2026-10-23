//! What the secondary windows (settings, Theme Studio, About, Licenses) share: window options, and for
//! the small ones a window that exists at most once and closes with Esc or ⌘W. Their surfaces and the
//! strip to drag them by are the style's (`style::window_surfaces`).

use gpui_kit::*;

use crate::app_state::AppState;
use crate::app_state::app_mut;
use crate::style;

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
            traffic_light_position: Some(crate::window::chrome::traffic_light_position(style::TITLE_ROW_H)),
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
