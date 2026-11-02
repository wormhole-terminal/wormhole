//! `kill` (SIGTERM) quits the app as ⌘Q does, without its question, so nothing unsaved is lost.
//!
//! Part of the app root (`app/`). The signal is heard on a thread of its own that blocks until it
//! arrives (core's `platform::quit_signal`), so the quit starts at once, not at the next tick.

use gpui_kit::*;
use wormhole_core::platform::quit_signal;

/// From now on, a SIGTERM quits the app. Needs [`quit_signal::catch_sigterm`] to have succeeded
/// (it didn't when the waiter fails at once: then a SIGTERM ends the app as it always did).
pub(super) fn quit_on_sigterm(cx: &mut App) {
    let (told, heard) = smol::channel::bounded::<()>(1);
    let spawned = std::thread::Builder::new().name("sigterm".into()).spawn(move || {
        if quit_signal::wait().is_ok() {
            // A full channel means the quit is already on its way.
            let _ = told.try_send(());
        }
    });
    if let Err(err) = spawned {
        tracing::warn!("SIGTERM will end the app without saving: {err}");
        return;
    }
    cx.spawn(async move |cx| {
        if heard.recv().await.is_ok() {
            tracing::info!("asked to quit (SIGTERM)");
            cx.update(|cx| cx.quit());
        }
    })
    .detach();
}
