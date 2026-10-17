//! One changed file as the Changes and History lists draw it: a
//! `style::file_row` whose name and folder are shortened to fit the panel, measured with the
//! interface font, the way `wormhole_core::elide` cuts them (the name in the middle, keeping
//! its extension; the folder at the front, keeping the folders nearest the file). Hovering a
//! row that had to be shortened shows the whole path.

use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::{self, FileLines, TextRole};
use wormhole_core::elide;
use wormhole_core::git::{ChangeKind, FileChange};

/// What a file row is drawn from besides the file.
pub(super) struct Placement {
    /// In a tree, the folder rows above say where the file is: the row shows its name only.
    pub in_tree: bool,
    pub depth: usize,
    pub selected: bool,
    /// The panel's width, which the name and folder are fitted to.
    pub width: f32,
}

/// The row for `file`; add `.on_click(…)`.
pub(super) fn change_row(
    file: &FileChange,
    place: Placement,
    ui: UiColors,
    window: &mut Window,
    cx: &App,
) -> Stateful<Div> {
    let Placement { in_tree, depth, selected, width } = place;
    let (folder, name) = match file.path.rsplit_once('/') {
        Some((folder, name)) => (if in_tree { "" } else { folder }, name),
        None => ("", file.path.as_str()),
    };
    let lines = if file.lfs {
        FileLines::Lfs
    } else {
        match (file.added, file.removed) {
            (Some(added), removed) => FileLines::Counts(added as usize, removed.unwrap_or(0) as usize),
            (None, _) => FileLines::Binary,
        }
    };
    let Renamed { from: renamed_from, moved } = renamed(file);
    let fitted = fit(name, folder, lines, renamed_from, moved, selected, width, depth, window, cx);
    let shortened = fitted.name != name || fitted.folder != folder;
    let icon_color = if selected { ui.selected_muted } else { ui.muted };
    let icon = crate::icons::file(name, icon_color, cx)
        .unwrap_or_else(|| style::icon(IconName::File).text_color(icon_color).into_any_element());
    let row = style::file_row(
        SharedString::from(format!("change-{depth}-{}", file.path)),
        ui,
        style::FileRow {
            icon,
            name: fitted.name,
            folder: fitted.folder,
            renamed_from: renamed_from.map(|old| SharedString::from(old.to_string())),
            moved,
            kind: file.kind,
            lines,
            depth,
            selected,
        },
    );
    // The whole path when the row had to cut it, and where a renamed file came from.
    if !shortened && file.kind != ChangeKind::Renamed {
        return row;
    }
    let (path, about) = (SharedString::from(file.path.clone()), about(file));
    let code_font = crate::fonts::code_font(cx).0;
    row.tooltip(move |window, cx| {
        let (path, about, code_font) = (path.clone(), about.clone(), code_font.clone());
        Tooltip::element(move |_, _| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .max_w(px(TOOLTIP_W))
                .child(div().font_family(code_font.clone()).child(path.clone()))
                .child(div().text_caption().text_color(ui.muted).child(about.clone()))
        })
        .build(window, cx)
    })
}

/// A path tooltip wraps at this width rather than running across the window.
const TOOLTIP_W: f32 = 280.0;

/// How a renamed file's row says where it came from: the old name before the new one
/// when the name changed, and a mark when the folder did. A `git mv` across folders that kept
/// the name shows the mark alone, where "a.rs → a.rs" would say nothing.
#[derive(Debug, PartialEq, Eq)]
struct Renamed<'a> {
    from: Option<&'a str>,
    moved: bool,
}

fn renamed(file: &FileChange) -> Renamed<'_> {
    let Some(old) = file.original_path.as_deref().filter(|_| file.kind == ChangeKind::Renamed) else {
        return Renamed { from: None, moved: false };
    };
    fn split(path: &str) -> (&str, &str) {
        path.rsplit_once('/').unwrap_or(("", path))
    }
    let (old_folder, old_name) = split(old);
    let (new_folder, new_name) = split(&file.path);
    Renamed { from: (old_name != new_name).then_some(old_name), moved: old_folder != new_folder }
}

/// The second line of a file's tooltip: "Added · +2210", "Renamed from src/old.rs · +1 −1".
fn about(file: &FileChange) -> SharedString {
    let kind = match file.kind {
        ChangeKind::Modified => "Modified",
        ChangeKind::Added => "Added",
        ChangeKind::Deleted => "Deleted",
        ChangeKind::Renamed => "Renamed",
        ChangeKind::Untracked => "Untracked",
        ChangeKind::Conflicted => "Conflicted",
    };
    let from = match (&file.original_path, file.kind) {
        (Some(old), ChangeKind::Renamed) => format!(" from {old}"),
        _ => String::new(),
    };
    let lines = match (file.lfs, file.added, file.removed) {
        (true, ..) => "LFS".to_string(),
        (false, None, _) => "binary".to_string(),
        (false, Some(added), Some(removed)) if removed > 0 => format!("+{added} −{removed}"),
        (false, Some(added), _) => format!("+{added}"),
    };
    format!("{kind}{from} · {lines}").into()
}

/// A file row's name and folder, as they fit.
struct Fitted {
    name: SharedString,
    folder: SharedString,
}

/// The gap between a row's parts (`gap_2`).
const GAP: f32 = 8.0;

/// Shorten `name` to [`style::FILE_NAME_MAX`] of the row and `folder` to what is left beside the
/// name, the counts and the kind letter.
#[allow(clippy::too_many_arguments)]
fn fit(
    name: &str,
    folder: &str,
    lines: FileLines,
    renamed_from: Option<&str>,
    moved: bool,
    selected: bool,
    width: f32,
    depth: usize,
    window: &mut Window,
    cx: &App,
) -> Fitted {
    let rem = window.rem_size();
    let family = cx.theme().font_family.clone();
    let text_system = window.text_system();
    let measure = |text: &str, size_rems: f32, weight: FontWeight| -> f32 {
        let run =
            TextRun { len: text.len(), font: Font { weight, ..font(family.clone()) }, ..Default::default() };
        f32::from(text_system.layout_line(text, rems(size_rems).to_pixels(rem), &[run], None).width)
    };
    let name_weight = if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL };
    let body = |text: &str| measure(text, BODY_REMS, name_weight);
    let caption = |text: &str| measure(text, CAPTION_REMS, FontWeight::NORMAL);

    let content = width - 2.0 * style::PANEL_ROW_X - depth as f32 * style::INDENT;
    let name = elide::fit(name, content * style::FILE_NAME_MAX, body, elide::middle);
    if folder.is_empty() {
        return Fitted { name: name.into_owned().into(), folder: SharedString::default() };
    }
    let counts = match lines {
        FileLines::Counts(added, removed) if removed > 0 => {
            caption(&format!("+{added}")) + 4.0 + caption(&format!("−{removed}"))
        }
        FileLines::Counts(added, _) => caption(&format!("+{added}")),
        FileLines::Binary => caption("bin"),
        FileLines::Lfs => caption("LFS"),
    };
    let renamed = renamed_from.map_or(0.0, |old| caption(old) + 4.0 + RENAME_ARROW + GAP);
    let mark = if moved { style::MOVED_MARK + GAP } else { 0.0 };
    // Lead, name, folder, spring, counts, letter: five gaps between six parts.
    let taken = style::ITEM_LEAD_W + body(&name) + renamed + mark + counts + style::KIND_LETTER_W + 5.0 * GAP;
    let folder = elide::fit(folder, content - taken, caption, elide::front);
    Fitted { name: name.into_owned().into(), folder: folder.into_owned().into() }
}

/// The type roles the row draws with, in rems: body (the name) and caption (folder, counts).
const BODY_REMS: f32 = 0.875;
const CAPTION_REMS: f32 = 0.75;
/// The arrow between a renamed file's old and new name.
const RENAME_ARROW: f32 = 12.0;

#[cfg(test)]
mod tests {
    use super::{Renamed, about, renamed};
    use wormhole_core::git::{ChangeKind, FileChange};

    fn file(kind: ChangeKind, added: Option<u32>, removed: Option<u32>, lfs: bool) -> FileChange {
        FileChange { path: "a/b.rs".into(), original_path: None, kind, added, removed, lfs }
    }

    fn renamed_file(from: &str, to: &str) -> FileChange {
        FileChange {
            path: to.into(),
            original_path: Some(from.into()),
            kind: ChangeKind::Renamed,
            added: Some(1),
            removed: Some(1),
            lfs: false,
        }
    }

    #[test]
    fn the_tooltip_says_what_happened_to_the_file() {
        assert_eq!(about(&file(ChangeKind::Added, Some(2210), Some(0), false)), "Added · +2210");
        assert_eq!(about(&file(ChangeKind::Modified, Some(128), Some(40), false)), "Modified · +128 −40");
        assert_eq!(about(&file(ChangeKind::Modified, None, None, false)), "Modified · binary");
        assert_eq!(about(&file(ChangeKind::Modified, Some(2), Some(2), true)), "Modified · LFS");
        // Where a renamed file came from, whole: the row only has room for the old name.
        assert_eq!(about(&renamed_file("src/old.rs", "lib/new.rs")), "Renamed from src/old.rs · +1 −1");
    }

    #[test]
    fn a_renamed_file_shows_its_old_name_and_a_moved_one_a_mark() {
        // A `git mv` to another folder that kept the name gets the mark, not "a.rs → a.rs".
        assert_eq!(renamed(&renamed_file("src/a.rs", "lib/a.rs")), Renamed { from: None, moved: true });
        assert_eq!(
            renamed(&renamed_file("src/old.rs", "src/new.rs")),
            Renamed { from: Some("old.rs"), moved: false }
        );
        assert_eq!(
            renamed(&renamed_file("workspace/panel.rs", "notes.rs")),
            Renamed { from: Some("panel.rs"), moved: true }
        );
        // Only a rename says where a file came from, whatever git recorded.
        let mut modified = file(ChangeKind::Modified, Some(1), Some(1), false);
        modified.original_path = Some("a/c.rs".into());
        assert_eq!(renamed(&modified), Renamed { from: None, moved: false });
    }
}
