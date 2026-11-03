//! Opening the settings window: where it goes on screen, and showing one setting's row from the palette.

use gpui_kit::*;
use wormhole_core::config::settings_index::{self};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::app_state::{app, app_mut};
use crate::aux_window;
use crate::settings::settings;
use crate::style::{self};

use super::{Section, SettingsWindow, WINDOW_MIN, WINDOW_SIZE};

/// The settings window while it is open, and its page; `None` once the window has closed. The shared
/// state keeps the page untyped and weak, so it doesn't name this window, which sits above it.
pub(super) fn current_window(cx: &App) -> Option<(AnyWindowHandle, Entity<SettingsWindow>)> {
    let (handle, page) = crate::app_state::live(&app(cx).settings_window)?;
    Some((handle, page.downcast::<SettingsWindow>().ok()?))
}

/// Bring up the settings window (showing `section`), or focus it if it is open.
pub(crate) fn open_settings(section: Option<Section>, cx: &mut App) {
    if super::native::enabled(cx) {
        super::native::open(section, cx);
        return;
    }
    // What the page lists of agent access is read again now, not from the last time.
    crate::agent_access::forget_listing(cx);
    if let Some((handle, page)) = current_window(cx) {
        let shown = handle.update(cx, |_, window, cx| {
            page.update(cx, |page, cx| {
                if let Some(section) = section {
                    page.show_section(section, cx);
                }
                cx.notify();
            });
            crate::background::activate_window(window, cx);
        });
        if shown.is_ok() {
            return;
        }
    }
    let mut options = aux_window::window_options(
        text(SETTINGS_WINDOW_SETTINGS),
        size(px(WINDOW_SIZE.0), px(WINDOW_SIZE.1)),
        size(px(WINDOW_MIN.0), px(WINDOW_MIN.1)),
        cx,
    );
    options.window_bounds =
        Some(WindowBounds::Windowed(Bounds::centered(None, size(px(WINDOW_SIZE.0), px(WINDOW_SIZE.1)), cx)));
    // Deferred: the new window renders right away, and this may run inside
    // another window's update.
    cx.defer(move |cx| {
        let opened = crate::window_open::open(options, cx, |window, cx| {
            cx.new(|cx| SettingsWindow::new(section, window, cx))
        });
        match opened {
            Ok((handle, page)) => app_mut(cx).settings_window = Some((handle, page.downgrade().into())),
            Err(err) => tracing::error!("opening the settings window failed: {err:#}"),
        }
    });
}

/// Bring up the settings window at the row `id` itself (the command palette): its
/// section, scrolled so the row sits a title row below the top, with that one row marked (a
/// search would mark every row with the same words). Until another section is picked or a search
/// starts.
pub(crate) fn reveal_row(id: &str, cx: &mut App) {
    let Some((section, _, entry)) = settings_index::entry(id) else { return };
    // Never open a section this build hides (a Preview section whose flag is off): a stale recent
    // id could still name one, and the page would only bounce back to Appearance.
    if !section.is_visible(settings(cx)) {
        return;
    }
    open_settings(Some(section), cx);
    // After `open_settings`'s own deferred opening, so the window exists by then.
    cx.defer(move |cx| {
        let Some((handle, page)) = current_window(cx) else { return };
        let shown = handle.update(cx, |_, window, cx| {
            page.update(cx, |page, cx| {
                page.clear_search(window, cx);
                page.show_section(section, cx);
                let anchor = ScrollAnchor::for_handle(page.content_scroll.clone());
                page.revealed = Some((entry.id, anchor));
                page.reveal_pending = true;
                cx.notify();
            });
            crate::background::activate_window(window, cx);
        });
        if let Err(err) = shown {
            tracing::warn!("revealing setting {}: {err:#}", entry.id);
        }
    });
}

/// How far below the page's top a revealed row lands: a title row, so it reads as the first
/// thing on the page rather than as cut off.
pub(super) const REVEAL_INSET: f32 = style::TITLE_ROW_H;
