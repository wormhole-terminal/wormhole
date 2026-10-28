//! Reading the open file off the main thread, and again when it changes.
//!
//! The file is read on GPUI's background executor, never where a frame is drawn (a slow volume must not
//! freeze the window). Its stamp (`FileStamp`: device, inode, size and both times, one `lstat`) is taken
//! before the read; every [`POLL_EVERY`](super::POLL_EVERY) ticks the stamp is taken again in the
//! background and the file is read only if it differs, so a file nobody touches costs one `lstat` and
//! redraws nothing. A file that is deleted is told so, and shows again if it comes back.
//! The syntax colors are parsed in a second background job once the text is in, so the text shows at once.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::*;
use wormhole_core::config::FeatureId;
use wormhole_core::storage::files::{self, FileStamp, ImageKind, Loaded, Problem};

use super::lines::{Lines, RowCache};
use super::{Content, POLL_EVERY, TextShown};
use crate::features::files::Files;
use crate::syntax::highlight::TextHighlights;
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
    Image { image: Option<Arc<Image>>, size: u64 },
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
        Loaded::Image { bytes, kind, size } => Found::Image { image: picture(bytes, kind), size },
        Loaded::Unavailable(problem) => Found::Unavailable(problem),
    };
    Some(Fetched { stamp, found })
}

/// The picture GPUI draws for a file's bytes; `None` when it can't be decoded.
fn picture(bytes: Vec<u8>, kind: ImageKind) -> Option<Arc<Image>> {
    let format = match kind {
        ImageKind::Png => ImageFormat::Png,
        ImageKind::Jpeg => ImageFormat::Jpeg,
        ImageKind::Gif => ImageFormat::Gif,
        ImageKind::Webp => ImageFormat::Webp,
        ImageKind::Heic => return super::image::from_heic(&bytes),
    };
    Some(Arc::new(Image::from_bytes(format, bytes)))
}

impl Files {
    /// Reads the open file now (a new one, or a look found it changed), in the background.
    pub(super) fn read_viewer(&mut self, cx: &mut Context<Self>) {
        self.poll_viewer(false, cx);
    }

    /// Looks at the open file in the background and reads it if it is not as it was read (`only_changed`),
    /// or in any case.
    pub(super) fn poll_viewer(&mut self, only_changed: bool, cx: &mut Context<Self>) {
        let Some(viewer) = self.viewer.as_mut().filter(|viewer| !viewer.reading) else { return };
        viewer.reading = true;
        let (id, root, path) = (viewer.id, viewer.root.clone(), viewer.path.clone());
        let prior = if only_changed { viewer.stamp } else { None };
        cx.spawn(async move |this, cx| {
            let (asked_root, asked) = (root.clone(), path.clone());
            let fetched = cx.background_spawn(async move { fetch(&asked_root, &asked, prior) }).await;
            this.update(cx, |files, cx| files.arrived_file(id, &path, fetched, cx)).ok();
        })
        .detach();
    }

    /// Once per window tick: the open file is looked at about every second, while the feature is on and
    /// the file's terminal is the one on screen. Nothing is read when the file looks as it did.
    pub(in crate::features::files) fn tick_viewer(
        &mut self,
        facts: &WindowFacts<'_>,
        cx: &mut Context<Self>,
    ) {
        if !facts.features.has(FeatureId::Files) || !facts.ticks.is_multiple_of(POLL_EVERY) {
            return;
        }
        if self.viewer.as_ref().is_some_and(|v| facts.active().is_some_and(|active| active.id == v.session)) {
            self.poll_viewer(true, cx);
        }
    }

    /// A read came back. It is dropped when the viewer has moved to another file since it was asked for.
    fn arrived_file(&mut self, id: u64, path: &PathBuf, fetched: Option<Fetched>, cx: &mut Context<Self>) {
        let Some(viewer) = self.viewer.as_mut().filter(|viewer| viewer.id == id) else { return };
        viewer.reading = false;
        // The file was renamed while it was read: what came back is of the old name.
        if &viewer.path != path {
            self.read_viewer(cx);
            return;
        }
        // Nothing was read: the file looks as it did.
        let Some(Fetched { stamp, found }) = fetched else { return };
        viewer.stamp = Some(stamp);
        let mut colour = false;
        viewer.content = match found {
            Found::Text { lines, size, truncated } => {
                colour = lines.len() > 0;
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
                    rows: Rc::new(RowCache::default()),
                })
            }
            Found::Binary { size } => Content::Binary { size },
            Found::Image { image, size } => Content::Image { image, size },
            Found::Unavailable(problem) => Content::Unavailable(problem),
        };
        if colour {
            self.colour_viewer(id, cx);
        }
        cx.notify();
    }

    /// Parses the syntax colors of the shown text in the background.
    fn colour_viewer(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(viewer) = self.viewer.as_ref().filter(|viewer| viewer.id == id) else { return };
        let Content::Text(shown) = &viewer.content else { return };
        let (text, name) = (shown.lines.text().clone(), viewer.path.to_string_lossy().into_owned());
        let lines = shown.lines.clone();
        cx.spawn(async move |this, cx| {
            let colors = cx
                .background_spawn(async move { TextHighlights::for_text(&text, &name).map(Arc::new) })
                .await;
            this.update(cx, |files, cx| files.coloured(id, &lines, colors, cx)).ok();
        })
        .detach();
    }

    /// The colors of the text that was parsed. They are dropped if the text has been read again since.
    fn coloured(
        &mut self,
        id: u64,
        lines: &Rc<Lines>,
        colors: Option<Arc<TextHighlights>>,
        cx: &mut Context<Self>,
    ) {
        let Some(viewer) = self.viewer.as_mut().filter(|viewer| viewer.id == id) else { return };
        let Content::Text(shown) = &mut viewer.content else { return };
        if colors.is_some() && Rc::ptr_eq(&shown.lines, lines) {
            shown.colors = colors;
            cx.notify();
        }
    }
}
