//! Wormhole as the Mac's default terminal: Settings → Terminal makes it the app that runs a
//! double-clicked `.command` or program (Launch Services, by content type), and the files and
//! folders macOS asks it to open each get a new terminal in the window in front. Which types, and
//! what a terminal does with each, are core's `default_terminal`; holding the role is
//! `platform::default_app`.

use gpui_kit::*;
use wormhole_core::terminal::default_terminal::{Opened, file_path};

use crate::workspace::app::AppState;

/// Hear what macOS asks to open, from the launch on: the requests wait in the channel until
/// [`serve`] takes them, as a launch to open a file sends it before the windows are there.
pub(crate) fn listen(application: &Application) -> smol::channel::Receiver<Vec<String>> {
    let (opened, requests) = smol::channel::unbounded();
    application.on_open_urls(move |urls| {
        let _ = opened.try_send(urls);
    });
    requests
}

/// Open each request in a new terminal, once the windows are up.
pub(crate) fn serve(requests: smol::channel::Receiver<Vec<String>>, cx: &mut App) {
    cx.spawn(async move |cx| {
        while let Ok(urls) = requests.recv().await {
            cx.update(|cx| open_all(&urls, cx));
        }
    })
    .detach();
}

fn open_all(urls: &[String], cx: &mut App) {
    for url in urls {
        let Some(path) = file_path(url) else {
            tracing::warn!(url, "asked to open something that isn't a local file");
            continue;
        };
        let Some(opened) = path.metadata().ok().and_then(|meta| Opened::of(&path, meta.is_dir())) else {
            tracing::warn!(path = %path.display(), "asked to open a file that can't be read");
            continue;
        };
        open(&opened, cx);
    }
}

/// A new terminal for `opened` in the window in front, which comes forward.
fn open(opened: &Opened, cx: &mut App) {
    let Some(target) = AppState::front_window(cx) else {
        tracing::warn!("asked to open {} with no window open", opened.folder().display());
        return;
    };
    let folder = opened.folder().to_str().map(str::to_string);
    let reached = AppState::with_workspace(&target, cx, |ws, window, cx| {
        ws.control_new_terminal(folder, opened.line(), window, cx);
        crate::background::activate_window(window, cx);
    });
    if reached.is_none() {
        tracing::warn!("the window for {} could not be reached", opened.folder().display());
        return;
    }
    crate::background::activate(cx);
    tracing::info!(folder = %opened.folder().display(), script = opened.line().is_some(), "opened from the system");
}
