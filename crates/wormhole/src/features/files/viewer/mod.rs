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
mod colors;
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
use crate::platform::image::Refused;
use crate::terminal_view::TerminalView;
use crate::window::feature::FeatureEvent;
use colors::Colors;
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
    /// A picture, decoded by the system (`image`), or why it was not.
    Image {
        image: Result<Arc<RenderImage>, Refused>,
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
    /// The syntax colors, once they are made (the text shows plain meanwhile).
    colors: Option<Arc<Colors>>,
    /// The syntax theme (by `colors::theme_key`) colors were last asked for: a file without a grammar has
    /// none, and is not asked again until the theme changes.
    coloured_for: Option<usize>,
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
            Content::Image { image, size } => match image {
                Ok(image) => {
                    let (w, h) = (image.size(0).width.0, image.size(0).height.0);
                    format!("image {size} bytes, shown at {w}x{h}")
                }
                Err(Refused::TooLarge { width, height }) => {
                    format!("image {size} bytes, {width}x{height} is too large")
                }
                Err(Refused::Unreadable) => format!("image {size} bytes, can't be decoded"),
            },
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
        self.replace_viewer(Some(viewer), cx);
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
        if self.viewer.is_some() {
            self.replace_viewer(None, cx);
            window.focus(&terminal.focus_handle(cx), cx);
            cx.notify();
        }
    }

    /// Takes the viewer down without touching the keyboard: another cover opened, or another terminal
    /// was selected.
    pub(super) fn dismiss_viewer(&mut self, cx: &mut Context<Self>) {
        if self.viewer.is_some() {
            self.replace_viewer(None, cx);
            cx.notify();
        }
    }

    /// Puts `viewer` in place of the one shown, and lets go of the old one's picture.
    fn replace_viewer(&mut self, viewer: Option<Viewer>, cx: &mut Context<Self>) {
        if let Some(old) = std::mem::replace(&mut self.viewer, viewer) {
            self.release_content(old.content, cx);
        }
    }

    /// `content` is no longer shown: a picture it holds is dropped from GPUI's atlas. GPUI keeps the
    /// tile it drew until it is told, so without this every picture looked at would stay in video memory.
    pub(super) fn release_content(&mut self, content: Content, cx: &mut Context<Self>) {
        if let Content::Image { image: Ok(image), .. } = content {
            self.release_image(image, cx);
        }
    }

    /// Drops `image`'s tile from the window's atlas, once the update that asked is over (the window may be
    /// in the middle of one now).
    fn release_image(&mut self, image: Arc<RenderImage>, cx: &mut Context<Self>) {
        let window = self.window;
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, _| {
                    window.drop_image(image).ok();
                })
                .ok();
        });
    }

    /// A terminal is gone from this window: its viewer goes with it.
    pub(super) fn forget_viewer(&mut self, session: i64, cx: &mut Context<Self>) {
        if self.viewer.as_ref().is_some_and(|v| v.session == session) {
            self.dismiss_viewer(cx);
        }
    }

    /// The row at `from` below `root` was renamed to `to`: the viewer follows the file, or a file inside
    /// the folder.
    pub(super) fn viewer_renamed(
        &mut self,
        session: i64,
        root: &Path,
        from: &Path,
        to: &Path,
        cx: &mut Context<Self>,
    ) {
        // The same terminal can have been pointed at another folder since the file was opened: a path
        // below one root says nothing about the other's.
        let Some(viewer) = self.viewer.as_mut().filter(|v| v.session == session && v.root == root) else {
            return;
        };
        if let Ok(rest) = viewer.path.strip_prefix(from) {
            viewer.path = to.join(rest);
            cx.notify();
        }
    }
}
