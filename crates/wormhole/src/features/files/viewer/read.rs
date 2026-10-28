//! Reading the open file off the main thread, and again when it changes.
//!
//! The file is read on GPUI's background executor, never where a frame is drawn (a slow volume must not
//! freeze the window). Its stamp (`FileStamp`: device, inode, size and both times, one `lstat`) is taken
//! before the read; every [`POLL_EVERY`](super::POLL_EVERY) ticks the stamp is taken again in the
//! background and the file is read only if it differs, so a file nobody touches costs one `lstat` and
//! redraws nothing. A file that is deleted is told so, and shows again if it comes back.
//!
//! One read is on its way at a time for the whole window, and the latest request wins: a read cannot be
//! cancelled (up to 50 MB and a decode), so walking the tree with the arrow keys asks for a file per row
//! but only starts the one that is wanted when the one before it comes back. The syntax colors are made
//! in a second background job, likewise one at a time, once the text is in, so the text shows at once.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::*;
use wormhole_core::config::FeatureId;
use wormhole_core::storage::files::{self, FileStamp, Loaded, Problem};

use super::colors::{Colors, theme_key};
use super::image::picture;
use super::lines::{Lines, RowCache};
use super::{Content, POLL_EVERY, TextShown};
use crate::features::files::Files;
use crate::platform::image::Refused;
use crate::settings::store_in;
use crate::window::facts::WindowFacts;

/// What a read found, made ready off the main thread.
pub(super) struct Fetched {
    /// How the file looked just before it was read.
    stamp: Option<FileStamp>,
    found: Found,
}

enum Found {
    Text { lines: Lines, size: u64, truncated: bool },
    Binary { size: u64 },
    Image { image: Result<Arc<RenderImage>, Refused>, size: u64 },
    Unavailable(Problem),
}

/// Reads the file `relative` below `root` unless its stamp is `prior` (a stamp, or none for a file that was
/// not there), which says nothing changed. Blocks on the disk.
pub(super) fn fetch(root: &Path, relative: &Path, prior: Option<Option<FileStamp>>) -> Option<Fetched> {
    // Taken before the read, so a change while it is read shows up as one the next time.
    let stamp = FileStamp::of_link(&root.join(relative));
    if prior == Some(stamp) {
        return None;
    }
    let found = match files::load(root, relative) {
        Loaded::Text { text, size, truncated } => Found::Text { lines: Lines::of(&text), size, truncated },
        Loaded::Binary { size } => Found::Binary { size },
        Loaded::Image { bytes, size, .. } => Found::Image { image: picture(&bytes), size },
        Loaded::Unavailable(problem) => Found::Unavailable(problem),
    };
    Some(Fetched { stamp, found })
}

impl Files {
    /// Reads the open file now (a new one, or a look found it changed), in the background.
    pub(super) fn read_viewer(&mut self, cx: &mut Context<Self>) {
        self.poll_viewer(false, cx);
    }

    /// Looks at the open file in the background and reads it if it is not as it was read (`only_changed`),
    /// or in any case. Nothing is started while a read is on its way: its answer starts the next one.
    pub(super) fn poll_viewer(&mut self, only_changed: bool, cx: &mut Context<Self>) {
        if self.fetching {
            return;
        }
        let Some(viewer) = self.viewer.as_ref() else { return };
        let (id, root, path) = (viewer.id, viewer.root.clone(), viewer.path.clone());
        let prior = if only_changed { viewer.stamp } else { None };
        self.fetching = true;
        #[cfg(test)]
        {
            self.started.0 += 1;
        }
        cx.spawn(async move |this, cx| {
            let (asked_root, asked) = (root.clone(), path.clone());
            let fetched = cx.background_spawn(async move { fetch(&asked_root, &asked, prior) }).await;
            this.update(cx, |files, cx| files.arrived_file(id, &path, fetched, cx)).ok();
        })
        .detach();
    }

    /// Once per window tick. The syntax colors follow the theme; and the open file is looked at about every
    /// second, while the feature is on and the file's terminal is the one on screen. Nothing is read when
    /// the file looks as it did, and nothing at all while the feature is off.
    pub(in crate::features::files) fn tick_viewer(
        &mut self,
        facts: &WindowFacts<'_>,
        cx: &mut Context<Self>,
    ) {
        if !facts.features.has(FeatureId::Files) {
            return;
        }
        self.colour_viewer(cx);
        if !facts.ticks.is_multiple_of(POLL_EVERY) {
            return;
        }
        if self.viewer.as_ref().is_some_and(|v| facts.active().is_some_and(|active| active.id == v.session)) {
            self.poll_viewer(true, cx);
        }
    }

    /// A read came back. The viewer may have moved to another file since it was asked for: then the answer
    /// is dropped and the file that is wanted now is read.
    fn arrived_file(&mut self, id: u64, path: &PathBuf, fetched: Option<Fetched>, cx: &mut Context<Self>) {
        self.fetching = false;
        let current = self.viewer.as_ref().is_some_and(|viewer| viewer.id == id && &viewer.path == path);
        if !current {
            // The latest request wins: whatever is shown now has not been read yet.
            if self.viewer.as_ref().is_some_and(|viewer| viewer.stamp.is_none()) {
                self.read_viewer(cx);
            }
            return;
        }
        // Nothing was read: the file looks as it did.
        let Some(Fetched { stamp, found }) = fetched else { return };
        let Some(viewer) = self.viewer.as_mut() else { return };
        viewer.stamp = Some(stamp);
        let content = match found {
            Found::Text { lines, size, truncated } => {
                // The reader may be scrolled far into a file that got shorter.
                let keep = matches!(&viewer.content, Content::Text(old) if old.lines.len() >= lines.len());
                if !keep {
                    viewer.scroll = UniformListScrollHandle::new();
                }
                Content::Text(TextShown {
                    lines: Rc::new(lines),
                    size,
                    truncated,
                    colors: None,
                    coloured_for: None,
                    rows: Rc::new(RowCache::default()),
                })
            }
            Found::Binary { size } => Content::Binary { size },
            Found::Image { image, size } => Content::Image { image, size },
            Found::Unavailable(problem) => Content::Unavailable(problem),
        };
        let old = std::mem::replace(&mut viewer.content, content);
        self.release_content(old, cx);
        self.colour_viewer(cx);
        cx.notify();
    }

    /// Makes the syntax colors of the shown text in the background, when it has none for the current
    /// theme. One job at a time: when it returns, a text or a theme that came meanwhile is done next.
    pub(super) fn colour_viewer(&mut self, cx: &mut Context<Self>) {
        if self.colouring {
            return;
        }
        let theme = store_in(&self.profile, cx).highlight.clone();
        let Some(viewer) = self.viewer.as_ref() else { return };
        let Content::Text(shown) = &viewer.content else { return };
        if shown.lines.len() == 0 || shown.coloured_for == Some(theme_key(&theme)) {
            return;
        }
        let (lines, name) = (shown.lines.clone(), viewer.path.to_string_lossy().into_owned());
        self.colouring = true;
        #[cfg(test)]
        {
            self.started.1 += 1;
        }
        let made = (lines.clone(), theme.clone());
        cx.spawn(async move |this, cx| {
            // `Rc` is not `Send`: the job takes the text itself (a shared string) and makes its own index.
            let text = made.0.text().clone();
            let theme_for_job: Arc<HighlightTheme> = made.1.clone();
            let colors = cx
                .background_spawn(async move {
                    let lines = Lines::of(&text);
                    Colors::make(&lines, &name, &theme_for_job)
                })
                .await;
            this.update(cx, |files, cx| files.coloured(&made.0, &made.1, colors, cx)).ok();
        })
        .detach();
    }

    /// The colors of a text came back. They are dropped if the viewer shows other text now; whatever is
    /// wanted next (another text, another theme) is started.
    fn coloured(
        &mut self,
        lines: &Rc<Lines>,
        theme: &Arc<HighlightTheme>,
        colors: Option<Colors>,
        cx: &mut Context<Self>,
    ) {
        self.colouring = false;
        if let Some(viewer) = self.viewer.as_mut()
            && let Content::Text(shown) = &mut viewer.content
            && Rc::ptr_eq(&shown.lines, lines)
        {
            shown.colors = colors.map(Arc::new);
            shown.coloured_for = Some(theme_key(theme));
            cx.notify();
        }
        self.colour_viewer(cx);
    }
}
