//! Links in the pane, as other macOS terminals do them: with ⌘ held, the link under the
//! pointer (a program's OSC 8 hyperlink or a URL in the text) is underlined and the pointer
//! becomes a hand; ⌘-click opens it, also in a program that takes the mouse. Which links may
//! open, and how, is core's `Openable`.

use std::ops::Range;

use gpui_kit::{App, Context, Pixels, Point};
use wormhole_term::types::Line;
use wormhole_term::{Link, Openable};

use super::{Layout, TerminalView};

impl TerminalView {
    /// The pointer moved, or ⌘ went down or up: underline the link under it while ⌘ is held.
    pub(super) fn hover_link(&mut self, position: Point<Pixels>, command: bool, cx: &mut Context<Self>) {
        let link = command.then(|| self.link_under(position)).flatten();
        if link != self.hovered_link {
            self.hovered_link = link;
            cx.notify();
        }
    }

    /// ⌘-click: open the link under the pointer. `false` when there's none there, so the
    /// click does what it would otherwise.
    pub(super) fn open_link(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        let Some(link) = self.link_under(position) else { return false };
        open_uri(&link.uri, cx);
        true
    }

    /// The address of the link under `position`, for the context menu.
    pub(super) fn link_uri_under(&self, position: Point<Pixels>) -> Option<String> {
        self.link_under(position).map(|link| link.uri)
    }

    fn link_under(&self, position: Point<Pixels>) -> Option<Link> {
        if !over_grid(&self.layout.get(), position) {
            return None;
        }
        let (point, _) = self.grid_point(position)?;
        self.pty.as_ref()?.link_at(point)
    }

    /// The hovered link's columns on each row of the viewport it shows on.
    pub(super) fn hovered_link_rows(&self, display_offset: usize) -> Vec<(usize, Range<usize>)> {
        let Some(link) = &self.hovered_link else { return Vec::new() };
        let layout = self.layout.get();
        (0..layout.rows)
            .filter_map(|row| {
                let line = Line(row as i32 - display_offset as i32);
                Some((row, link.columns_on(line, layout.cols)?))
            })
            .collect()
    }
}

/// Open a link the way ⌘-click does: only the kinds core allows, a refused one is logged.
pub(super) fn open_uri(uri: &str, cx: &mut App) {
    match Openable::from_uri(uri) {
        Some(Openable::Url(url)) => cx.open_url(&url),
        Some(Openable::Reveal(path)) => cx.reveal_path(&path),
        None => {
            // The scheme only: the rest of a link can be private.
            let scheme = uri.split(':').next().unwrap_or_default();
            tracing::info!(scheme, "a terminal link wasn't opened: not a kind that's allowed");
        }
    }
}

/// Whether `position` is on a cell (`grid_point` takes the nearest cell for one beside the grid,
/// as a selection dragged past the edge needs).
fn over_grid(layout: &Layout, position: Point<Pixels>) -> bool {
    let x = position.x - layout.origin.x;
    let y = position.y - layout.origin.y;
    x >= Pixels::ZERO
        && y >= Pixels::ZERO
        && x < layout.cell.width * layout.cols as f32
        && y < layout.cell.height * layout.rows as f32
}
