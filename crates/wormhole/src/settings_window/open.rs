//! Opening the settings window: where it goes on screen (and how it grows when the preview column opens),
//! and showing one setting's row from the palette.

use gpui_kit::*;
use wormhole_core::config::settings_index::{self};

use super::preview_column::PREVIEW_W;
use crate::app_state::{app, app_mut};
use crate::aux_window;
use crate::settings::settings;
use crate::style::{self};

use super::{Section, SettingsWindow, WINDOW_MIN, WINDOW_SIZE};

/// The settings window while it is open, and its page. The shared state keeps the page untyped, so it
/// doesn't name this window, which sits above it.
pub(super) fn current_window(cx: &App) -> Option<(AnyWindowHandle, Entity<SettingsWindow>)> {
    let (handle, page) = app(cx).settings_window.clone()?;
    Some((handle, page.downcast::<SettingsWindow>().ok()?))
}

/// Bring up the settings window (showing `section`), or focus it if it is open.
pub(crate) fn open_settings(section: Option<Section>, cx: &mut App) {
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
        "Settings",
        size(px(WINDOW_SIZE.0), px(WINDOW_SIZE.1)),
        size(px(WINDOW_MIN.0), px(WINDOW_MIN.1)),
        cx,
    );
    options.window_bounds = Some(WindowBounds::Windowed(opening_bounds(cx)));
    // Deferred: the new window renders right away, and this may run inside
    // another window's update.
    cx.defer(move |cx| {
        let opened = crate::background::open_window(options, cx, |window, cx| {
            cx.new(|cx| SettingsWindow::new(section, window, cx))
        });
        match opened {
            Ok((handle, page)) => app_mut(cx).settings_window = Some((handle, page.into())),
            Err(err) => tracing::error!("opening the settings window failed: {err:#}"),
        }
    });
}

/// Where a new window opens: centred as a window of [`WINDOW_SIZE`] would be, then grown to the
/// right by the preview column, so the page stays where it was. A screen too narrow for that
/// (the Dock, a small display) moves it left and then narrows it, to stay inside the usable area;
/// below [`COLUMN_SWITCH_W`] the previews are inline in their pages.
fn opening_bounds(cx: &App) -> Bounds<Pixels> {
    let page = Bounds::centered(None, size(px(WINDOW_SIZE.0), px(WINDOW_SIZE.1)), cx);
    match cx.primary_display() {
        Some(display) => grown_right(page, PREVIEW_W, display.visible_bounds()),
        None => Bounds { size: size(page.size.width + px(PREVIEW_W), page.size.height), ..page },
    }
}

/// `page` with `extra` more width on its right: its left edge stays, unless the screen's usable
/// area `screen` is too narrow for that, which moves the window left, and then narrows it.
fn grown_right(page: Bounds<Pixels>, extra: f32, screen: Bounds<Pixels>) -> Bounds<Pixels> {
    let width = (page.size.width + px(extra)).min(screen.size.width);
    let overflow = (page.origin.x + width - (screen.origin.x + screen.size.width)).max(px(0.0));
    let x = (page.origin.x - overflow).max(screen.origin.x);
    Bounds { origin: point(x, page.origin.y), size: size(width, page.size.height) }
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

#[cfg(test)]
mod tests {
    use super::grown_right;
    use gpui_kit::{Bounds, Pixels, point, px, size};

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds { origin: point(px(x), px(y)), size: size(px(w), px(h)) }
    }

    #[test]
    fn a_window_grows_to_the_right_and_keeps_its_left_edge() {
        let grown = grown_right(bounds(260.0, 40.0, 920.0, 720.0), 480.0, bounds(0.0, 25.0, 2560.0, 1400.0));
        assert_eq!(grown, bounds(260.0, 40.0, 1400.0, 720.0));
    }

    #[test]
    fn a_screen_too_narrow_moves_it_left_then_narrows_it() {
        let screen = bounds(0.0, 25.0, 1440.0, 875.0);
        // Room for the width but not from x 260: it moves left until its right edge fits.
        assert_eq!(
            grown_right(bounds(260.0, 40.0, 920.0, 720.0), 480.0, screen),
            bounds(40.0, 40.0, 1400.0, 720.0)
        );
        // Not even the width fits: the usable width, from the screen's left edge.
        let small = bounds(0.0, 25.0, 1280.0, 775.0);
        assert_eq!(
            grown_right(bounds(180.0, 40.0, 920.0, 720.0), 480.0, small),
            bounds(0.0, 40.0, 1280.0, 720.0)
        );
    }
}
