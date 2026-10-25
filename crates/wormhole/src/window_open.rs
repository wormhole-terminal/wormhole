//! Opening a window: the one way the app does it, so every window gets what all of them need.
//! `gpui_kit::open_window` itself is called nowhere else (`tests/background_rules.rs` checks).
//!
//! What every window gets: while background mode keeps Wormhole behind the other apps, it opens
//! without the focus and behind their windows (`background`), and its traffic lights, if it has
//! any, are drawn in the theme's appearance (`platform::window::follow_theme`).

use gpui_kit::*;

/// Open a window of the app and build its root view.
pub fn open<V: Render>(
    mut options: WindowOptions,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> anyhow::Result<(AnyWindowHandle, Entity<V>)> {
    let behind = crate::background::keeps_new_windows_behind(cx);
    if behind {
        crate::background::open_unfocused(&mut options);
    }
    let (handle, view) = gpui_kit::open_window(options, cx, build)?;
    let _ = handle.update(cx, |_, window, _| crate::platform::window::follow_theme(window));
    if behind {
        let _ = handle.update(cx, |_, window, _| crate::background::send_behind(window));
    }
    Ok((handle, view))
}
