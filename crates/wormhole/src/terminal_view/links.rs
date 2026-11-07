//! Links in the pane, as other macOS terminals do them: with ⌘ held, the link under the
//! pointer (a program's OSC 8 hyperlink or a URL in the text) is underlined and the pointer
//! becomes a hand; ⌘-click opens it, also in a program that takes the mouse. Which links may
//! open, and how, is core's `Openable`.
//!
//! A Probe's rules (`probe_runtime`) add two kinds: a link to an address, which goes through `Openable` like
//! any other, and a file (`path:line:col`), which is looked for in the terminal's folder through the
//! hardened reads of `storage::files` and opened in the editor Open in… names, never run. While ⌘ is held
//! over one, a chip says where it goes and which Probe said so. The same rules may shorten or redact a span;
//! the pointer over it shows the real text again (`update_reveal`), when its rule allows.

use std::ops::Range;

use gpui_kit::{App, Context, IntoElement, Modifiers, ParentElement, Pixels, Point, Styled, div};
use wormhole_core::probe::open_path::{self, PathRef};
use wormhole_term::types::Line;
use wormhole_term::{Link, Openable, RevealWhen};

use super::{Layout, TerminalView};

/// What the pointer reveals: the span it is over, and the cell and ⌥ it was last looked up for.
#[derive(Default)]
pub(super) struct Reveal {
    /// The viewport cell, `(row, column)`, where the revealed span starts.
    pub(super) at: Option<(usize, usize)>,
    cell: Option<(usize, usize, bool)>,
}

impl TerminalView {
    /// The pointer moved, or ⌘ went down or up: underline the link under it while ⌘ is held.
    pub(super) fn hover_link(&mut self, position: Point<Pixels>, command: bool, cx: &mut Context<Self>) {
        let link = command.then(|| self.link_under(position, cx)).flatten();
        if link != self.hovered_link {
            self.hovered_link = link;
            cx.notify();
        }
    }

    /// ⌘-click: open the link under the pointer. `false` when there's none there, so the
    /// click does what it would otherwise.
    pub(super) fn open_link(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        let Some(link) = self.link_under(position, cx) else { return false };
        match link.rule {
            Some(rule) if rule.file => self.open_path(link.uri, cx),
            _ => open_uri(&link.uri, cx),
        }
        true
    }

    /// The address of the link under `position`, for the context menu. A file a rule found is no address.
    pub(super) fn link_uri_under(&self, position: Point<Pixels>, cx: &App) -> Option<String> {
        self.link_under(position, cx)
            .filter(|link| link.rule.as_ref().is_none_or(|rule| !rule.file))
            .map(|link| link.uri)
    }

    fn link_under(&self, position: Point<Pixels>, cx: &App) -> Option<Link> {
        if !over_grid(&self.layout.get(), position) {
            return None;
        }
        let (point, _) = self.grid_point(position)?;
        let pty = self.pty.as_ref()?;
        match crate::probe_runtime::rules(cx) {
            Some(rules) => pty.link_at_with(point, &*rules),
            None => pty.link_at(point),
        }
    }

    /// ⌘-click on a file a rule named (`text` is `path[:line[:col]]`): look for it in the terminal's folder
    /// off the main thread, then ask the editor to open it. The folder is the shell's own (its prompt told it,
    /// or its process is in it); a path out of it, or through a link that leaves it, is not opened.
    fn open_path(&self, text: String, cx: &mut Context<Self>) {
        let (Some(target), Some(cwd)) = (PathRef::parse(&text), self.cwd.clone()) else {
            tracing::info!("a file a Probe named was not opened: no folder to look in, or not a path");
            return;
        };
        let line = target.line;
        let found =
            cx.background_executor().spawn(async move { open_path::resolve(&[cwd.as_path()], &target) });
        cx.spawn(async move |_, cx| match found.await {
            Ok(path) => cx.update(|cx| open_in_editor(&path, line, cx)),
            Err(err) => tracing::info!("a file a Probe named was not opened: {err}"),
        })
        .detach();
    }

    /// The pointer moved (or ⌥ changed): show the real text of the shortened or redacted span under it, when
    /// its rule allows. Nothing is looked up while no rules are in force, and while the pointer stays in the
    /// cell it was in.
    pub(super) fn update_reveal(
        &mut self,
        position: Point<Pixels>,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) {
        let cell = crate::probe_runtime::rules(cx)
            .and_then(|_| self.viewport_cell(position))
            .map(|(col, row)| (col, row, modifiers.alt));
        if cell == self.reveal.cell && self.reveal.cell.is_some() {
            return;
        }
        self.reveal.cell = cell;
        let at = cell.and_then(|(_, _, alt)| self.reveal_at(position, alt, cx));
        if at != self.reveal.at {
            self.reveal.at = at;
            cx.notify();
        }
    }

    /// The pointer left the pane: nothing is shown revealed.
    pub(super) fn end_reveal(&mut self, cx: &mut Context<Self>) {
        self.reveal.cell = None;
        if self.reveal.at.take().is_some() {
            cx.notify();
        }
    }

    /// The viewport cell where the masked span under `position` starts, if the rule lets it show.
    fn reveal_at(&self, position: Point<Pixels>, alt: bool, cx: &App) -> Option<(usize, usize)> {
        let rules = crate::probe_runtime::rules(cx)?;
        let (point, _) = self.grid_point(position)?;
        let span = self.pty.as_ref()?.rule_span_at(point, &*rules)?;
        let allowed = match span.hit.reveal {
            RevealWhen::Hover => true,
            RevealWhen::Deliberate => alt,
            RevealWhen::Never => false,
        };
        let offset = self.render_state.borrow().display_offset() as i32;
        let row = usize::try_from(span.start.line.0 + offset).ok()?;
        (span.hit.masked && allowed).then_some((row, span.start.column.0))
    }

    /// Where the hovered rule link goes, for the chip: what the rule says of it, the target and the Probe.
    /// The target is always shown: a rule's words must not stand in for where the link really goes.
    pub(super) fn render_link_readout(&self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        let link = self.hovered_link.as_ref()?;
        let rule = link.rule.as_ref()?;
        let ui = crate::settings::store_in(&self.profile, cx).ui;
        let name = crate::probe_runtime::probe_name(&rule.owner, cx).unwrap_or_else(|| rule.owner.clone());
        let mut text = String::new();
        if let Some(hover) = &rule.hover {
            text.push_str(hover);
            text.push_str(" → ");
        }
        text.push_str(&wormhole_core::probe::hidden::reveal(&link.uri));
        text.push_str(" · ");
        text.push_str(&wormhole_core::probe::hidden::reveal(&name));
        Some(
            div()
                .absolute()
                .bottom_2()
                .left_2()
                .max_w_full()
                .overflow_hidden()
                .child(crate::style::size_readout(ui, text, 1.0))
                .into_any_element(),
        )
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

/// Ask the editor Open in… names to open the file `path` (a real path of a regular file) at `line`.
fn open_in_editor(path: &std::path::Path, line: Option<u32>, cx: &App) {
    let bundle = crate::open_in::OpenIn::resolved(cx).app.bundle;
    crate::open_in::open_file_in(&bundle, path, line);
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
