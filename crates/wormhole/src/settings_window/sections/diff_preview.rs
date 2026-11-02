//! Changes & diffs' preview: the Changes panel and a diff beside it, compiled from the design's `DiffMock`
//! (`ui/generated/diff_mock.rs`) and filled from the page's options: list or tree, untracked files apart or
//! not, split or unified, wrapped or cut, the tab's width. The miniature's rows are the design's; this
//! file only chooses the made-up files and the diff's lines.

use gpui_kit::*;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::card::{Words, card};
use crate::settings::{SettingsStore, settings};
use crate::settings_window::{SettingsWindow, ui};
use crate::theme_bridge::token;
use crate::ui::{Cell, DiffLine, DiffMockProps, File, Look};

/// How a file of the panel stands against the repository.
#[derive(Clone, Copy)]
enum Mark {
    Modified,
    Added,
    Untracked,
}

/// A file of the sample panel: its folder (empty at the top), its name and its mark.
struct Sample {
    folder: &'static str,
    name: &'static str,
    mark: Mark,
}

// text: sample (made-up files of the preview)
const TRACKED: [Sample; 3] = [
    Sample { folder: "src", name: "limits.rs", mark: Mark::Modified },
    Sample { folder: "src", name: "lib.rs", mark: Mark::Modified },
    Sample { folder: "tests", name: "limits.rs", mark: Mark::Added },
];
// text: sample (a made-up file of the preview)
const UNTRACKED: Sample = Sample { folder: "", name: "notes.txt", mark: Mark::Untracked };

/// Changes & diffs → the panel and a diff as the options make them.
pub(in crate::settings_window) fn draw_diff_preview(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let s = settings(cx).clone();
    let tokens = &cx.global::<SettingsStore>().tokens;
    let (warning, added, muted) =
        (token(tokens, "status.warning.text"), token(tokens, "agent.done"), token(tokens, "text.muted"));
    let tree = s.changes_tree;
    let badge = |mark: Mark| match mark {
        Mark::Modified => ("M", warning),
        Mark::Added => ("A", added),
        Mark::Untracked => ("U", muted),
    };
    let file = |sample: &Sample, deep: bool| {
        let (badge, color) = badge(sample.mark);
        File {
            name: if tree || sample.folder.is_empty() {
                sample.name.into()
            } else {
                format!("{}/{}", sample.folder, sample.name).into()
            },
            badge: badge.into(),
            badge_color: color,
            deep,
            dir: false,
        }
    };
    let dir = |name: &str| File { name: name.to_owned().into(), dir: true, ..File::default() };
    let mut files = Vec::new();
    let mut folder = "";
    for sample in &TRACKED {
        if tree && sample.folder != folder {
            folder = sample.folder;
            files.push(dir(&format!("{folder}/")));
        }
        files.push(file(sample, tree));
    }
    if s.group_untracked {
        files.push(dir(text(SETTINGS_GIT_PREVIEW_UNTRACKED)));
    }
    files.push(file(&UNTRACKED, tree && s.group_untracked));

    let indent = " ".repeat(s.tab_width);
    // text: sample (the lines of a made-up diff)
    let long = format!("{indent}let budget = self.refill(now).min(self.capacity * BURST_FACTOR);");
    let (wrap, split) = (s.diff_wrap, s.diff_split);
    let cell = |text: String, left: bool, kind: &str| Cell {
        text: text.into(),
        wrap,
        left,
        del: kind == "del",
        add: kind == "add",
        none: kind == "none",
    };
    let cells = if split {
        // text: sample (the lines of a made-up diff)
        [
            ("fn take(&mut self) {".to_owned(), "fn take(&mut self) {".to_owned(), "", ""),
            (format!("{indent}let budget = self.tokens;"), long.clone(), "del", "add"),
            (format!("{indent}if budget == 0 {{"), format!("{indent}if budget == 0 {{"), "", ""),
            (String::new(), format!("{indent}{indent}self.stats.denied += 1;"), "none", "add"),
        ]
        .into_iter()
        .flat_map(|(l, r, lk, rk)| [cell(l, true, lk), cell(r, false, rk)])
        .collect()
    } else {
        Vec::new()
    };
    let line = |text: String, kind: &str| DiffLine {
        text: text.into(),
        wrap,
        del: kind == "del",
        add: kind == "add",
    };
    let lines = if split {
        Vec::new()
    } else {
        // text: sample (the lines of a made-up diff)
        vec![
            line(" fn take(&mut self) {".to_owned(), ""),
            line(format!("-{indent}let budget = self.tokens;"), "del"),
            line(format!("+{long}"), "add"),
            line(format!(" {indent}if budget == 0 {{"), ""),
            line(format!("+{indent}{indent}self.stats.denied += 1;"), "add"),
        ]
    };
    let mock = DiffMockProps {
        id: "diff-mock".into(),
        spoken: text(SETTINGS_WINDOW_PREVIEW_DIFF_LABEL).into(),
        heading: format!("{} · {}", text(SETTINGS_GIT_PREVIEW_CHANGES), TRACKED.len() + 1).into(),
        path: "src/limits.rs".into(), // text: sample (a made-up file of the preview)
        split,
        unified: !split,
        files,
        cells,
        lines,
    };
    card(
        ui,
        Words {
            id: "diff-preview-card",
            what: SETTINGS_WINDOW_DIFF,
            spoken: SETTINGS_WINDOW_PREVIEW_DIFF_LABEL,
            caption: SETTINGS_GIT_PREVIEW_CAPTION,
        },
        div().child(mock.render(&Look::of(cx))).overflow_hidden(),
    )
}
