//! Opening a window: the one way the app does it, so every window gets what all of them need.
//! `gpui_kit::open_window` itself is called nowhere else (`tests/background_rules.rs` checks).
//!
//! What every window gets: while background mode keeps Wormhole behind the other apps, it opens
//! without the focus and behind their windows (`background`), and its traffic lights, if it has
//! any, are drawn in the theme's appearance (`platform::window::follow_theme`).
//!
//! A window that opens with the system's animation (a terminal window, `window::scaling`) is opened out of
//! sight, with `show: false`, and put on the screen by [`reveal`] once its first frame is drawn.

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

/// A window that was opened with `show: false` for the system's animation: give it that animation (it closes
/// with it too), draw its first frame while it is out of sight, then put it on the screen with the keys, so
/// the animation never shows an empty box. Done after the current update, as drawing needs the app free; if
/// the window cannot be drawn ahead, it is put on the screen as any window opens.
pub fn reveal(handle: AnyWindowHandle, cx: &mut App) {
    let _ = handle.update(cx, |_, window, _| {
        crate::platform::window::set_animation(
            window,
            wormhole_core::view::motion::window_animation::Behaviour::System,
        );
    });
    cx.spawn(async move |cx| {
        let ready = cx.update(|cx| {
            handle.update(cx, |_, window, _| crate::platform::window::Native::of(window)).ok().flatten()
        });
        match ready {
            Some(ready) => {
                ready.paint();
                ready.show(true);
            }
            None => cx.update(|cx| {
                let _ = handle.update(cx, |_, window, _| crate::platform::window::show(window));
            }),
        }
        // As a window that opens in front does: the app comes forward with it (unless background mode). And a
        // frame once more, in case Metal did not present the first one for a window that was out of sight.
        cx.update(|cx| {
            let _ = handle.update(cx, |_, window, cx| {
                window.refresh();
                crate::background::activate_window(window, cx);
            });
        });
    })
    .detach();
}
