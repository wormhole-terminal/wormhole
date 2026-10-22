//! What the diff card shows for a submodule in place of lines: the commits between the two
//! commits the parent records (like `git diff --submodule=log`), newest first, one row each with its
//! subject, short hash, author and age; the commits the pointer left behind when it went back; or
//! why there is nothing to list (not checked out, not fetched).

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::DiffView;
use crate::settings::UiColors;
use crate::style::{self, TextRole};
use crate::window::Workspace;
use wormhole_core::git::{COMMITS_SHOWN, Commit, Range, SubmoduleLog, short};
use wormhole_core::view::limits::count_grouped;
use wormhole_core::view::panel::age;

/// The room round the card's notes.
const NOTE_PAD: f32 = 16.0;

impl Workspace {
    /// The body of a submodule's card. `view` has the submodule's own change; `log` what is
    /// between its commits.
    pub(super) fn render_submodule_body(
        &self,
        view: &DiffView,
        log: &SubmoduleLog,
        ui: UiColors,
        cx: &App,
    ) -> AnyElement {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let code_font = crate::fonts::code_font(cx).0;
        let sub = view.file().submodule.as_ref();
        let mut body = div().id("submodule-body").size_full().flex().flex_col().overflow_y_scroll();

        // Which commit to which, in the code font: what the pointer was and what it is.
        if let (Some(old), Some(new)) = (&log.old, &log.new)
            && old != new
        {
            body = body.child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .text_caption()
                    .text_color(ui.muted)
                    .font_family(code_font.clone())
                    .child(short(old).to_string())
                    .child(style::icon(gpui_kit::assets::IconName::ArrowRight))
                    .child(short(new).to_string()),
            );
        }
        if let Some(note) = log.nothing_to_list() {
            body = body.child(note_line(ui, note));
        }
        if let Range::Commits { ahead, behind, ahead_total, behind_total } = &log.range {
            if !ahead.is_empty() {
                body = body.children(commit_section(
                    ui,
                    "New commits",
                    ahead,
                    *ahead_total,
                    now,
                    code_font.clone(),
                ));
            }
            if !behind.is_empty() {
                body = body.children(commit_section(
                    ui,
                    "Commits left behind",
                    behind,
                    *behind_total,
                    now,
                    code_font.clone(),
                ));
            }
        }
        if let Some(note) = sub.and_then(|sub| sub.inside_note()) {
            body = body.child(note_line(ui, note));
        }
        body.into_any_element()
    }
}

/// A sentence in muted text.
#[track_caller]
fn note_line(ui: UiColors, text: String) -> Div {
    div().flex_none().px_4().py(px(NOTE_PAD / 2.0)).text_body().text_color(ui.muted).child(text)
}

/// A heading with the number of commits, the commits, and "and 340 more" after the ones that are
/// shown.
fn commit_section(
    ui: UiColors,
    title: &str,
    commits: &[Commit],
    total: usize,
    now: i64,
    code_font: SharedString,
) -> Vec<AnyElement> {
    let heading = div()
        .flex()
        .flex_none()
        .items_center()
        .gap_2()
        .px_4()
        .pt_3()
        .pb_1()
        .child(style::section_label(ui, title))
        .child(style::count_badge(ui, count_grouped(total)))
        .into_any_element();
    let rows = commits.iter().map(|commit| commit_line(ui, commit, now, code_font.clone()));
    let more = (total > commits.len().min(COMMITS_SHOWN)).then(|| {
        note_line(ui, format!("and {} more", count_grouped(total - commits.len()))).into_any_element()
    });
    std::iter::once(heading).chain(rows).chain(more).collect()
}

/// One commit: its subject, and under it the short hash, the author and how long ago.
#[track_caller]
fn commit_line(ui: UiColors, commit: &Commit, now: i64, code_font: SharedString) -> AnyElement {
    div()
        .id(SharedString::from(format!("submodule-commit-{}", commit.sha)))
        .flex()
        .flex_none()
        .flex_col()
        .justify_center()
        .w_full()
        .h(px(style::ROW_TWO_LINE_H))
        .px_4()
        .py_1()
        .child(
            div()
                .flex()
                .items_center()
                .h(rems(style::NAME_LINE))
                .text_body()
                .child(div().min_w_0().truncate().child(commit.subject.clone())),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .h(rems(style::META_LINE))
                .text_caption()
                .text_color(ui.muted)
                .child(
                    div()
                        .flex_none()
                        .text_color(ui.primary)
                        .font_family(code_font)
                        .child(commit.short.clone()),
                )
                .child(div().min_w_0().truncate().child(commit.author.clone()))
                .child(div().flex_1())
                .child(div().flex_none().tabular_nums().child(age(now - commit.time))),
        )
        .into_any_element()
}
