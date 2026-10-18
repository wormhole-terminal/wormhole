//! The right panel's parts: section bands, task and note rows,
//! file, folder and commit rows, the Project tab's two-line rows and trust callout, icon tabs
//! and line counts.

use std::rc::Rc;

use super::*;
use gpui_kit::component::switch::Switch;
use style::{BandKind, BandParts, FileLines, FileRow, ItemParts, Section};
use wormhole_core::git::ChangeKind;
use wormhole_core::project::{Feature, FeatureStanding};

/// The panel's slices are drawn 300 wide (narrower in a narrow gallery column).
const COLUMN_W: f32 = 300.0;

pub(super) fn render(studio: &StudioWindow, env: &Env, cx: &mut Context<StudioWindow>) -> AnyElement {
    let ui = env.ui;
    let section = |id: &'static str, title: &'static str, section, count: Option<&'static str>| {
        style::band(
            id,
            ui,
            BandParts {
                kind: BandKind::Section,
                section,
                title: title.into(),
                detail: None,
                count: count.map(SharedString::from),
                waiting: None,
                add: Some(("Add", Box::new(|_, _, _| {}))),
                on_toggle: Some(Box::new(|_, _, _| {})),
                drop_target: false,
                sticky: false,
                joined: false,
            },
        )
    };
    let actions = |id: &'static str| {
        vec![
            style::item_action((id, 0usize), ui, IconName::ArrowUpRight, "Send to terminal", |_, _, _| {}),
            style::item_action((id, 1usize), ui, IconName::X, "Delete", |_, _, _| {}),
        ]
    };
    let check = |id: &'static str, done: bool| style::checkbox(id, ui, done).into_any_element();
    let task = |id: &'static str, label: &'static str, done: bool, reorder: Option<usize>| {
        let main = style::item_label(ui, label, done).when(done, |d| d.line_through());
        style::item_row(
            id,
            ui,
            ItemParts {
                lead: check(id, done),
                main: main.into_any_element(),
                actions: actions(id),
                body: None,
                active: false,
                reorder: reorder.map(|ix| style::Reorder {
                    dragged: false,
                    list: "g-flush-tasks".into(),
                    ix,
                    label: label.into(),
                    on_move: Rc::new(|_, _, _, _| {}),
                }),
            },
        )
    };
    let note = |id: &'static str, title: &'static str, body: Option<&'static str>| {
        style::item_row(
            id,
            ui,
            ItemParts {
                lead: style::disclosure(ui, body.is_none()).into_any_element(),
                main: style::item_label(ui, title, body.is_some()).into_any_element(),
                actions: actions(id),
                body: body.map(|text| div().px_2().text_body().child(text).into_any_element()),
                active: false,
                reorder: None,
            },
        )
    };
    let file = |id: &'static str, name: &'static str, folder: &'static str, kind, lines, selected, depth| {
        style::file_row(
            id,
            ui,
            FileRow {
                icon: style::icon(IconName::File).into_any_element(),
                name: name.into(),
                folder: folder.into(),
                renamed_from: (kind == ChangeKind::Renamed).then(|| "panel.rs".into()),
                moved: kind == ChangeKind::Renamed,
                kind,
                lines,
                depth,
                selected,
            },
        )
    };

    let notes = div()
        .flex()
        .flex_col()
        .w_full()
        .max_w(px(COLUMN_W))
        .bg(ui.panel_right)
        .outlined(ui, ui.border)
        .child(env.mark("panel.band", section("g-sec-steps", "Next steps", Section::Open, Some("2"))))
        .child(env.mark("panel.item", task("g-flush-task", "Update README with setup steps", false, Some(0))))
        .child(task("g-flush-done", "Fix the flaky login test", true, Some(1)))
        // Being edited: the editor in place of the label, the fill and the actions stay on.
        .child(style::item_row(
            "g-flush-editing",
            ui,
            ItemParts {
                lead: check("g-flush-editing-check", false),
                main: style::item_editor(Input::new(&studio.item_editor).appearance(false).small())
                    .into_any_element(),
                actions: actions("g-flush-editing"),
                body: None,
                active: true,
                reorder: None,
            },
        ))
        // More done steps than five fold into one row after the open ones.
        .child(
            style::flush_row("g-done-fold", ui, false)
                .child(style::lead(style::disclosure(ui, true)))
                .child(div().text_color(ui.muted).child("12 done")),
        )
        .child(section("g-sec-notes", "Notes", Section::Open, Some("2")))
        .child(note("g-flush-note-closed", "Release checklist", None))
        .child(note(
            "g-flush-note-open",
            "Sidebar ideas",
            Some("Drop the idle line on quiet rows.\nBranch as a chip next to the folder name."),
        ))
        .child(section("g-sec-folded", "Notes", Section::Collapsed, Some("2")))
        .child(section("g-sec-empty", "Notes", Section::Empty, None));

    let changes = div()
        .flex()
        .flex_col()
        .w_full()
        .max_w(px(COLUMN_W))
        .bg(ui.panel_right)
        .outlined(ui, ui.border)
        .child(section("g-sec-changes", "Changes", Section::Open, Some("6")))
        .child(file(
            "g-file-m",
            "style.rs",
            "crates/wormhole/src",
            ChangeKind::Modified,
            FileLines::Counts(6, 4),
            false,
            0,
        ))
        .child(env.mark(
            "file.row",
            file(
                "g-file-sel",
                "sidebar.rs",
                "…/workspace",
                ChangeKind::Modified,
                FileLines::Counts(17, 10),
                true,
                0,
            ),
        ))
        .child(file(
            "g-file-u",
            "chip.rs",
            "crates/wormhole/src/style",
            ChangeKind::Untracked,
            FileLines::Counts(42, 0),
            false,
            0,
        ))
        .child(file(
            "g-file-d",
            "old_panel.rs",
            "workspace",
            ChangeKind::Deleted,
            FileLines::Counts(0, 212),
            false,
            0,
        ))
        .child(file("g-file-r", "notes.rs", "", ChangeKind::Renamed, FileLines::Counts(1, 1), false, 0))
        .child(file("g-file-bin", "logo.png", "assets", ChangeKind::Modified, FileLines::Binary, false, 0))
        .child(file("g-file-lfs", "demo.mov", "assets", ChangeKind::Modified, FileLines::Lfs, false, 0))
        // A name too long for its share of the row loses its middle.
        .child(file(
            "g-file-long",
            "integrat…_arm64.snap",
            "…shots",
            ChangeKind::Added,
            FileLines::Counts(2210, 0),
            false,
            0,
        ))
        // A folder of the tree view, and a file one level in.
        .child(style::folder_row(
            "g-file-folder",
            ui,
            0,
            style::icon(IconName::Folder).into_any_element(),
            "workspace",
            false,
        ))
        .child(file(
            "g-file-tree",
            "sidebar.rs",
            "",
            ChangeKind::Modified,
            FileLines::Counts(17, 10),
            false,
            1,
        ));

    let commit = |id: &'static str, subject: &'static str, sha: &'static str, age: &'static str, expanded| {
        style::commit_row(
            id,
            ui,
            style::CommitRow {
                subject: subject.into(),
                sha: sha.into(),
                author: "Benny".into(),
                age: age.into(),
                code_font: crate::fonts::code_font(cx).0,
                expanded,
            },
        )
    };
    let history = div()
        .flex()
        .flex_col()
        .w_full()
        .max_w(px(COLUMN_W))
        .bg(ui.panel_right)
        .outlined(ui, ui.border)
        .child(env.mark(
            "commit.row",
            commit("g-commit-open", "feat(ui): unify design system across panels", "adcd556", "30m", true),
        ))
        .child(file(
            "g-commit-file",
            "tasks.rs",
            "workspace",
            ChangeKind::Modified,
            FileLines::Counts(413, 120),
            false,
            1,
        ))
        .child(commit("g-commit", "chore(dev): add watch-and-relaunch dev script", "19601d4", "31m", false));

    let two_line = |id: &'static str, icon, title: &'static str, detail: SharedString, warn: bool| {
        style::two_line_row(
            id,
            ui,
            style::TwoLineRow {
                lead: style::icon(icon).text_color(ui.muted).into_any_element(),
                title: title.into(),
                detail: Some(detail),
                detail_color: warn.then_some(ui.warning_text),
                trailing: Some(Switch::new(id).checked(true).into_any_element()),
                actions: Vec::new(),
            },
        )
    };
    let hint = |standing: FeatureStanding, feature| {
        SharedString::from(standing.hint(feature).unwrap_or_else(|| feature.description().to_string()))
    };
    let code = crate::fonts::code_font(cx).0;
    let project = div()
        .flex()
        .flex_col()
        .w_full()
        .max_w(px(COLUMN_W))
        .bg(ui.panel_right)
        .outlined(ui, ui.border)
        // The trust callout: full width, the commands in the code font, one primary button.
        .child(
            env.mark(
                "callout",
                style::callout(
                    ui,
                    style::BannerKind::Warning,
                    "Trust this project?",
                    [
                        div()
                            .text_caption()
                            .text_color(ui.muted)
                            .child("It wants to run these commands. Nothing runs until you approve.")
                            .into_any_element(),
                        div()
                            .p_2()
                            .rounded(ui.radius)
                            .bg(ui.hover)
                            .text_caption()
                            .font_family(code)
                            .child("cargo test --workspace")
                            .into_any_element(),
                        Button::new("g-trust")
                            .primary()
                            .small()
                            .label("Trust with Touch ID")
                            .into_any_element(),
                    ],
                ),
            ),
        )
        .child(section("g-sec-use", "Use from this project", Section::Open, None))
        .child(two_line(
            "g-feature-settings",
            IconName::Settings,
            "Settings",
            "Setting overrides and shell".into(),
            false,
        ))
        // A file that sets nothing yet reads in the plain colour; what trust holds back, in the warning one.
        .child(two_line(
            "g-feature-layout",
            IconName::SquareTerminal,
            "Terminal layout",
            hint(FeatureStanding::Empty, Feature::Layout),
            false,
        ))
        .child(two_line(
            "g-feature-actions",
            IconName::Play,
            "Actions",
            hint(FeatureStanding::NeedsTrust, Feature::Actions),
            true,
        ))
        .child(style::two_line_row(
            "g-prompt",
            ui,
            style::TwoLineRow {
                lead: style::icon(IconName::File).text_color(ui.muted).into_any_element(),
                title: "Review the diff".into(),
                detail: Some("Point out bugs, then simplifications".into()),
                detail_color: None,
                trailing: None,
                actions: actions("g-prompt"),
            },
        ));

    let tabs = div()
        .flex()
        .items_center()
        .gap_1()
        .child(style::icon_tab("g-ftab-notes", ui, IconName::StickyNote, "Notes", false, None))
        .child(style::icon_tab("g-ftab-changes", ui, IconName::GitCompare, "Changes", false, Some(3)))
        .child(env.mark(
            "icon.tab",
            style::icon_tab("g-ftab-history", ui, IconName::GitCommitHorizontal, "History", true, Some(3)),
        ))
        .child(div().pl_4().child(style::line_counts(ui, Some(17), Some(10))));

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(tabs)
        .child(div().flex().flex_wrap().gap_4().child(notes).child(changes).child(history).child(project))
        .into_any_element()
}
