//! The small icon that opens a file of the git panel in the editor Open in… uses: on a Changes or
//! History row and on a diff line while the pointer is over it, and in the diff's header. A click
//! asks that app to open the file (at the line, where the app can: `wormhole_core::platform::open_file`);
//! for Finder it reveals the file.
//!
//! As for Open in… itself, the person asks an app they chose to open a file, so nothing from the
//! project runs and project trust (`ProjectHub::enabled`) doesn't come into it. What the path
//! names is checked first, though: repository content is hostile, so a path that leads out of
//! the repository (a planted symlink) is refused (`fsutil::resolve_within`).

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::{self, IconButton};
use crate::window::open_in::OpenIn;
use wormhole_core::platform::open_file;
use wormhole_core::platform::open_in::Installed;
use wormhole_core::storage::fsutil;
use wormhole_core::view::spoken;

/// The repository, and the app a click opens its files in: worked out once per frame of a list,
/// not per row.
#[derive(Clone)]
pub(crate) struct OpenTarget(Rc<Target>);

struct Target {
    root: PathBuf,
    app: Installed,
}

impl OpenTarget {
    pub(crate) fn new(root: &Path, cx: &App) -> Self {
        Self(Rc::new(Target { root: root.to_path_buf(), app: OpenIn::resolved(cx).app }))
    }

    /// One file of the repository, by its path relative to the root.
    pub(crate) fn file(&self, path: &str) -> OpenFile {
        OpenFile { target: self.clone(), path: path.to_string().into() }
    }
}

#[derive(Clone)]
pub(crate) struct OpenFile {
    target: OpenTarget,
    path: SharedString,
}

impl OpenFile {
    fn tooltip(&self, line: Option<u32>) -> String {
        let app = &self.target.0.app;
        open_file::tooltip(&app.bundle, &app.name, line)
    }

    fn open(&self, line: Option<u32>) {
        let Target { root, app } = &*self.target.0;
        open(&app.bundle, root, &self.path, line);
    }

    fn button(&self, id: impl Into<ElementId>, ui: UiColors, line: Option<u32>) -> IconButton {
        let tooltip = self.tooltip(line);
        let name = self.path.rsplit_once('/').map_or(&*self.path, |(_, name)| name);
        let spoken = spoken::open_file(&tooltip, name);
        let this = self.clone();
        style::icon_button(id, ui, IconName::ExternalLink, tooltip).spoken(spoken).on_click(
            move |_, _, cx| {
                // The click is the icon's, not the row's (which opens the diff).
                cx.stop_propagation();
                this.open(line);
            },
        )
    }

    /// The icon in a diff's header, always there, beside the other controls.
    pub(crate) fn header_button(&self, ui: UiColors) -> IconButton {
        self.button("diff-open-in-editor", ui, None).in_title_row()
    }

    /// `row` with the icon at its right end while the pointer is over the row: it takes no room
    /// until then (`style::row_actions`), when the text before it gives way.
    pub(crate) fn on_row(&self, row: Stateful<Div>, ui: UiColors, id: &str) -> Stateful<Div> {
        let group = SharedString::from(format!("open-row-{id}"));
        let action = self.button(SharedString::from(format!("open-{id}")), ui, None).in_row();
        row.group(group.clone()).child(style::row_actions(
            SharedString::from(format!("open-actions-{id}")),
            group,
            false,
            vec![action.into_any_element()],
        ))
    }

    /// The icon at the right end of a diff line, opening the file at `line`, while the pointer is
    /// over the line (`group`). `height` is the line's: the icon is clipped to it.
    pub(crate) fn on_line(
        &self,
        row: Stateful<Div>,
        ui: UiColors,
        group: &'static str,
        (id, box_id): (ElementId, ElementId),
        line: u32,
        height: f32,
    ) -> Stateful<Div> {
        let action = self.button(id, ui, Some(line)).in_row();
        row.relative().group(group).child(
            div()
                .absolute()
                .top_0()
                .right_3()
                .h(px(height))
                .flex()
                .items_center()
                .overflow_hidden()
                .child(style::row_actions(box_id, group, false, vec![action.into_any_element()])),
        )
    }
}

/// Open `relative` (a path of the repository at `root`) in the app with this bundle identifier.
pub(crate) fn open(bundle: &str, root: &Path, relative: &str, line: Option<u32>) {
    let path = match fsutil::resolve_within(root, relative) {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!("not opening {relative} from {}: {err}", root.display());
            return;
        }
    };
    let Some(launch) = open_file::launch(bundle, &path, line) else { return };
    if let Err(err) = crate::platform::apps::launch(bundle, launch) {
        tracing::warn!("opening {} in {bundle}: {err}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::open;

    #[test]
    fn a_path_that_leads_out_of_the_repository_starts_nothing() {
        // `open` would fail the test by launching something on a Mac; nothing is spawned for these.
        let root = std::env::temp_dir();
        for relative in ["../x", "/etc/hosts", "oie-no-such-file.rs"] {
            open("com.example.NotInstalled", &root, relative, Some(1));
        }
    }
}
