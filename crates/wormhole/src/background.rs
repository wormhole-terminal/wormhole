//! Background mode (`WORMHOLE_BACKGROUND=1`, devctl builds only): the test instances that
//! scripts and agents start (`scripts/devctl-instance.sh`, the visual check) open behind the other
//! apps' windows and never take the focus, so they never come over the work of whoever is at the
//! Mac. Wormhole then never activates itself and has no Dock icon (an accessory app); its windows
//! open without the focus, ordered behind every other window at the normal level; notifications
//! go to devctl's log, and the Dock, the bell's sound, VoiceOver's announcements, the global hotkey
//! and the status item stay out of the system. Once the person brings the app forward themselves
//! (a click on one of its windows), it behaves as usual until another app is in front again.
//! devctl's `front` brings a window forward on purpose.
//!
//! macOS stops drawing a window that others cover completely (GPUI stops its display link), and a
//! window kept behind everything often is covered: [`start`] draws those windows itself, so
//! screenshots, clicks and `wait-idle` see what a visible window would show.

use gpui_kit::*;

/// Whether this instance runs in background mode; never outside devctl builds, so the shipped app
/// can't be put there by its environment. The platform's functions ask it themselves.
pub fn on() -> bool {
    crate::platform::background_mode::on()
}

/// Whether Wormhole may bring itself or one of its windows forward: always, except in background
/// mode while another app is in front.
pub fn comes_forward(background: bool, app_active: bool) -> bool {
    !background || app_active
}

fn may_come_forward(cx: &App) -> bool {
    comes_forward(on(), crate::platform::dock::app_is_active(cx))
}

/// Bring Wormhole to the front (`cx.activate(true)`), unless background mode keeps it behind.
pub fn activate(cx: &mut App) {
    if may_come_forward(cx) {
        cx.activate(true);
    }
}

/// Bring `window` to the front and give it the keys, unless background mode keeps it behind.
pub fn activate_window(window: &Window, cx: &App) {
    if may_come_forward(cx) {
        window.activate_window();
    }
}

/// `gpui_kit::open_window`, except that while background mode keeps Wormhole behind, the window
/// opens without the focus and behind every other app's windows (a pop-up's level too).
pub fn open_window<V: Render>(
    mut options: WindowOptions,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> anyhow::Result<(AnyWindowHandle, Entity<V>)> {
    let behind = !may_come_forward(cx);
    if behind {
        // Not shown either: GPUI would order it in front, and it would flash over everything
        // until it is sent back.
        options.focus = false;
        options.show = false;
    }
    let (handle, view) = gpui_kit::open_window(options, cx, build)?;
    // Every window the app opens with lights has them drawn in the theme's appearance.
    let _ = handle.update(cx, |_, window, _| crate::platform::window::follow_theme(window));
    if behind {
        let _ = handle.update(cx, |_, window, _| crate::platform::window::keep_behind(window));
    }
    Ok((handle, view))
}

/// Set up background mode, if asked for: no Dock icon, and the covered windows drawn. Call it
/// first thing as the app starts, before any window opens.
pub fn start(cx: &mut App) {
    if !on() {
        return;
    }
    tracing::info!("background mode: windows open behind others and never take the focus");
    crate::platform::dock::hide_icon();
    #[cfg(feature = "devctl")]
    pump::start(cx);
    #[cfg(not(feature = "devctl"))]
    let _ = cx;
}

/// Frames for the windows macOS stopped drawing (see the module's comment).
#[cfg(feature = "devctl")]
mod pump {
    use std::time::Duration;

    use gpui_kit::*;

    /// How often a covered window is drawn: often enough that a batch of devctl commands (every
    /// 400 ms) always finds the window drawn as it is, seldom enough to stay cheap.
    const FRAME: Duration = Duration::from_millis(50);

    pub(super) fn start(cx: &mut App) {
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor().timer(FRAME).await;
                cx.update(draw_covered);
            }
        })
        .detach();
    }

    /// What the display link would do for each covered window: run what waits for the next frame
    /// (animations, lists measuring their rows), then draw. GPUI keeps whether a window is dirty
    /// to itself, so a covered window is drawn every time; the frame is presented once the window
    /// shows again.
    fn draw_covered(cx: &mut App) {
        for handle in cx.windows() {
            let _ = handle.update(cx, |_, window, cx| {
                if !window.is_visible() {
                    window.simulate_next_frame(cx);
                    window.draw(cx).clear(cx);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::comes_forward;

    #[test]
    fn background_mode_comes_forward_only_once_the_person_brought_it_there() {
        assert!(comes_forward(false, false), "the usual app activates itself");
        assert!(comes_forward(false, true));
        assert!(!comes_forward(true, false), "behind the app in front, it stays there");
        assert!(comes_forward(true, true), "brought forward by a click, it acts as usual");
    }
}
