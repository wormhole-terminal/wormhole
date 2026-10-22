//! A submodule's row in the Changes list and under a History commit: `style::submodule_row` drawn
//! from the file's change, with the button that opens a terminal in its folder.

use std::path::Path;

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::file_line::Placement;
use crate::settings::UiColors;
use crate::style::{self, TextRole};
use crate::window::Workspace;
use wormhole_core::git::FileChange;

/// A tooltip wraps at this width rather than running across the window.
const TOOLTIP_W: f32 = 280.0;

/// The row for `file`, a submodule; add `.on_click(…)`. `root` is the repository it is in.
pub(super) fn submodule_row(
    file: &FileChange,
    place: Placement,
    root: &Path,
    this: WeakEntity<Workspace>,
    ui: UiColors,
    cx: &App,
) -> Stateful<Div> {
    let Some(sub) = &file.submodule else { return div().id("not-a-submodule") };
    let Placement { in_tree, depth, selected, .. } = place;
    let (folder, name) = match file.path.rsplit_once('/') {
        Some((folder, name)) => (if in_tree { "" } else { folder }, name),
        None => ("", file.path.as_str()),
    };
    let what = SharedString::from(sub.label(file.kind));
    // Entering needs a folder with a repository in it; a removed submodule's is gone or about to be.
    let enter =
        (sub.checked_out() && !matches!(file.kind, wormhole_core::git::ChangeKind::Deleted)).then(|| {
            let (folder, this) = (root.join(&file.path), this);
            style::Enter {
                tooltip: format!("Open a terminal in {name}").into(),
                on_click: Box::new(move |_, window, cx| {
                    let folder = folder.clone();
                    let cwd = Some(folder.to_string_lossy().into_owned());
                    this.update(cx, |this, cx| {
                        this.open_session(None, cwd, window, cx);
                    })
                    .ok();
                }),
            }
        });
    let row = style::submodule_row(
        SharedString::from(format!("change-{depth}-{}", file.path)),
        ui,
        style::SubmoduleRow {
            name: name.to_string().into(),
            folder: folder.to_string().into(),
            what: what.clone(),
            kind: file.kind,
            depth,
            selected,
            enter,
        },
    )
    .aria_label(format!("Submodule {name}, {what}"));
    let (path, code_font) = (SharedString::from(file.path.clone()), crate::fonts::code_font(cx).0);
    row.tooltip(move |window, cx| {
        let (path, what, code_font) = (path.clone(), what.clone(), code_font.clone());
        Tooltip::element(move |_, _| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .max_w(px(TOOLTIP_W))
                .child(div().font_family(code_font.clone()).child(path.clone()))
                .child(div().text_caption().text_color(ui.muted).child(what.clone()))
        })
        .build(window, cx)
    })
}
