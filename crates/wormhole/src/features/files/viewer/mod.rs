//! The viewer: a file of the tree shown in the terminal's place, the way a diff is (`window::feature`'s
//! `CoverDef`; one cover at a time). A click on a file in the tree, or ↩ on it, opens it here; Esc, the
//! close button, another terminal or another cover closes it. One viewer per window, and opening another
//! file replaces it.
//!
//! What it shows is core's to decide (`storage::files::load`: text under a cap, binary, a picture, or
//! why the file can't be shown), read off the main thread by `read`, which also looks at the file
//! every so often and reads it again when its stamp changed. `lines` indexes the text so a long file is
//! a list of the rows in view; `card` draws the card, its header and the panels for what is not text.

mod card;
mod image;
mod lines;
mod read;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::*;
use wormhole_core::config::FeatureId;
use wormhole_core::storage::files::{FileStamp, Problem};
use wormhole_core::view::file_tree::{Row, RowKind};

use super::Files;
use crate::syntax::highlight::TextHighlights;
use crate::terminal_view::TerminalView;
use crate::window::feature::FeatureEvent;
use lines::{Lines, RowCache};

/// How many ticks (400 ms each) pass between two looks at the open file: about 0.8 s.
pub(super) const POLL_EVERY: u64 = 2;

/// The open file.
pub(super) struct Viewer {
    /// Counts the files opened: an answer for an earlier one is dropped.
    id: u64,
    /// The terminal it was opened from.
    pub(super) session: i64,
    /// The folder the tree was of, and the file below it.
    pub(super) root: PathBuf,
    pub(super) path: PathBuf,
    /// How the file looked from outside before it was last read (`None`: not read yet; `Some(None)`: it
    /// was not there).
    stamp: Option<Option<FileStamp>>,
    /// A read is on its way.
    reading: bool,
    pub(super) content: Content,
    scroll: UniformListScrollHandle,
    /// How far the code is scrolled sideways; the line numbers stay put.
    scroll_x: f32,
    /// The farthest `scroll_x` can go, from the last frame.
    max_scroll_x: Cell<f32>,
    /// Width of the rows' area in the last frame.
    body_width: Rc<Cell<f32>>,
    /// The axis the trackpad gesture in progress keeps to.
    gesture: OngoingScroll,
}

/// What the viewer has to show.
pub(super) enum Content {
    /// The first read is on its way.
    Reading,
    Text(TextShown),
    /// Not text: the panel says so, with the size.
    Binary {
        size: u64,
    },
    /// A picture. `None`: the system could not decode it.
    Image {
        image: Option<Arc<Image>>,
        size: u64,
    },
    Unavailable(Problem),
}

/// A text file.
pub(super) struct TextShown {
    lines: Rc<Lines>,
    /// The file's size, and whether `lines` holds only its start.
    size: u64,
    truncated: bool,
    /// The syntax colors, once they are parsed (the text shows plain meanwhile).
    colors: Option<Arc<TextHighlights>>,
    rows: Rc<RowCache>,
}

impl Viewer {
    fn new(id: u64, session: i64, root: PathBuf, path: PathBuf) -> Self {
        Self {
            id,
            session,
            root,
            path,
            stamp: None,
            reading: false,
            content: Content::Reading,
            scroll: UniformListScrollHandle::new(),
            scroll_x: 0.0,
            max_scroll_x: Cell::new(0.0),
            body_width: Rc::default(),
            gesture: OngoingScroll::default(),
        }
    }

    /// The file's name, as the tree names it.
    pub(super) fn name(&self) -> String {
        self.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
    }

    /// The file as a row of the tree, for what a row's menu does with a file.
    pub(super) fn as_row(&self) -> Row {
        Row { path: self.path.clone(), depth: 0, label: self.name(), kind: RowKind::File }
    }

    /// The text the viewer shows, for the tests and devctl.
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn describe(&self) -> String {
        let shown = match &self.content {
            Content::Reading => "reading".to_string(),
            Content::Text(text) => format!(
                "text {} lines, {} bytes{}{}",
                text.lines.len(),
                text.size,
                if text.truncated { ", cut" } else { "" },
                if text.colors.is_some() { ", colored" } else { "" }
            ),
            Content::Binary { size } => format!("binary {size} bytes"),
            Content::Image { image, size } => {
                format!("image {size} bytes{}", if image.is_some() { "" } else { ", can't be decoded" })
            }
            Content::Unavailable(problem) => format!("unavailable: {problem:?}"),
        };
        format!("{} ({shown})", self.path.display())
    }
}

impl Files {
    /// Shows the file `relative` below the tree's root in the terminal's place, and selects its row. A
    /// file that is shown already stays as it is.
    pub(super) fn show_file(&mut self, session: i64, relative: &Path, cx: &mut Context<Self>) {
        let Some(root) = self.of.get(&session).and_then(|of| of.root.clone()) else { return };
        let of = self.of_mut(session);
        if of.selected.as_deref() != Some(relative) {
            of.selected = Some(relative.to_path_buf());
        }
        if self.viewer.as_ref().is_some_and(|v| v.session == session && v.path == relative && v.root == root)
        {
            cx.notify();
            return;
        }
        self.viewers += 1;
        let viewer = Viewer::new(self.viewers, session, root, relative.to_path_buf());
        self.viewer = Some(viewer);
        // The window shows one cover at a time: a diff that was up comes down.
        cx.emit(FeatureEvent::CoverOpened(FeatureId::Files));
        self.read_viewer(cx);
        cx.notify();
    }

    /// Whether a file is shown.
    pub(super) fn viewing(&self) -> bool {
        self.viewer.is_some()
    }

    /// Takes the viewer down and gives `terminal`, which it covered, the keyboard.
    pub(super) fn close_viewer(
        &mut self,
        terminal: &Entity<TerminalView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.viewer.take().is_some() {
            window.focus(&terminal.focus_handle(cx), cx);
            cx.notify();
        }
    }

    /// Takes the viewer down without touching the keyboard: another cover opened, or another terminal
    /// was selected.
    pub(super) fn dismiss_viewer(&mut self, cx: &mut Context<Self>) {
        if self.viewer.take().is_some() {
            cx.notify();
        }
    }

    /// A terminal is gone from this window: its viewer goes with it.
    pub(super) fn forget_viewer(&mut self, session: i64, cx: &mut Context<Self>) {
        if self.viewer.as_ref().is_some_and(|v| v.session == session) {
            self.dismiss_viewer(cx);
        }
    }

    /// The row at `from` below the root was renamed to `to`: the viewer follows the file, or a file inside
    /// the folder.
    pub(super) fn viewer_renamed(&mut self, session: i64, from: &Path, to: &Path, cx: &mut Context<Self>) {
        let Some(viewer) = self.viewer.as_mut().filter(|v| v.session == session) else { return };
        if let Ok(rest) = viewer.path.strip_prefix(from) {
            viewer.path = to.join(rest);
            cx.notify();
        }
    }
}
