//! How the History tab draws: the log as a card of commits, the opened commit's details (`details_view`) and
//! files under it, and what stands in for the list (git is missing, the folder is no repository, still reading).

use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::git::{Commit, FileChange};
use wormhole_core::view::limits::count_grouped;
use wormhole_core::view::panel::age;

use super::def::typed;
use super::measure;
use super::menu::commit_menu;
use super::shown::Fact;
use super::state::CommitInfo;
use super::{CommitFiles, History, HistoryRow};
use crate::features::git::changes::Changes;
use crate::features::git::file_line::{Placement, change_row};
use crate::features::git::open_file::OpenTarget;
use crate::features::git::picker::{ChipOf, PickerChip};
use crate::features::git::problem::{self, GitTab};
use crate::features::git::submodule_row::{open_terminal_by_event, submodule_row};
use crate::platform;
use crate::settings::{UiColors, ui_in};
use crate::style::{self, TextRole};
use crate::window::facts::WindowFacts;

/// The tab's content, built inline from `Workspace::render` on the feature's entity.
pub(super) fn tab(
    entity: &AnyEntity,
    facts: &WindowFacts<'_>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    typed(entity).update(cx, |history, cx| history.render_tab(facts, window, cx))
}

impl History {
    fn render_tab(&mut self, facts: &WindowFacts<'_>, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.follow(facts);
        let Some(active) = facts.active() else { return div().into_any_element() };
        let (session, terminal) = (active.id, active.terminal);
        let ui = ui_in(facts.profile, cx);
        let panel = div().flex().flex_col().flex_1().min_h_0();
        // The rows are rebuilt for this frame's log; the entity is not notified, this draw is the redraw.
        let Some(state) = self.of.get_mut(&session) else { return div().into_any_element() };
        let rows = state.rows();
        state.list.set(rows);
        // A parent the person followed from the details: scroll to its row, now that the list has it.
        if let Some(sha) = state.reveal.take()
            && let Some(row) = state.list.items().iter().position(
                |row| matches!(row, HistoryRow::Commit(ix) if state.commits.get(*ix).is_some_and(|c| c.sha == sha)),
            )
        {
            state.list.reveal(row);
        }
        let history = &*state;

        if history.root.is_none() {
            if let Some(problem) = &history.problem {
                let this = cx.weak_entity();
                let retry = move |cx: &mut App| {
                    this.update(cx, |history, cx| history.refresh(session, cx)).ok();
                };
                return panel
                    .child(style::panel_card(
                        ui,
                        problem::render_git_problem(facts.profile, GitTab::History, problem, retry, cx),
                    ))
                    .into_any_element();
            }
            let answered = active.folder().is_some_and(|cwd| history.answered_for(cwd));
            let text = if answered {
                "This terminal's folder isn't inside a git repository."
            } else {
                "Reading history…"
            };
            return panel.child(style::panel_card(ui, style::empty_note(ui, text))).into_any_element();
        }

        let changes = facts.feature::<Changes>();
        let view = HistoryView {
            commits: history.commits.clone(),
            expanded: history.expanded.clone(),
            expanded_commit: history
                .expanded
                .as_ref()
                .and_then(|sha| history.commits.iter().find(|c| &c.sha == sha).cloned()),
            body_expanded: history.body_expanded,
            stat: history.expanded.as_ref().and_then(|sha| history.changes.get(sha).cloned()),
            info: history.expanded.as_ref().and_then(|sha| history.details.get(sha).cloned()),
            files: history.expanded.as_ref().and_then(|sha| history.loaded(sha).cloned()),
            present: history.expanded.as_ref().and_then(|sha| history.present.get(sha).cloned()),
            open: history.root.as_deref().map(|root| OpenTarget::new(root, cx)),
            files_error: history.expanded.as_ref().and_then(|sha| match history.files.get(sha) {
                Some(CommitFiles::Failed(err)) => Some(err.clone()),
                _ => None,
            }),
            error: history.error.clone(),
            root: history.root.clone(),
            open_path: changes
                .as_ref()
                .and_then(|changes| changes.read(cx).diff.as_ref().map(|diff| diff.file().path.clone())),
            code_font: crate::fonts::code_font_in(facts.profile, cx).0,
            now: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            width: f32::from(facts.panel_width),
            picker: self.picker.chip(
                cx.weak_entity(),
                ChipOf {
                    session,
                    terminal,
                    prefix: "",
                    branch: history.rev.as_deref(),
                    current: history.rev.clone(),
                },
                ui.branch,
            ),
            changes,
            session,
            this: cx.weak_entity(),
        };

        let profile = facts.profile.clone();
        let ring = facts.panel_cursor.map(|(row, shown)| (row, shown.then_some(ui)));
        // A column that fills the panel, so the list gets a bounded height.
        panel
            .child(history.list.with_edges(
                history.list.element_carded(ring, ui, HistoryRow::part, move |row, window, cx| {
                    view.render_row(row, ui_in(&profile, cx), window, cx)
                }),
                ui,
                "history-scrollbar",
                cx,
            ))
            .into_any_element()
    }
}

/// Where a commit's files and the lines about them start: one level in (x 40).
pub(super) const FILES_X: f32 = style::CARD_INSET + style::INDENT;

/// What the History list's rows draw from, captured for the render callback.
pub(super) struct HistoryView {
    pub(super) commits: Rc<Vec<Commit>>,
    pub(super) expanded: Option<String>,
    pub(super) expanded_commit: Option<Commit>,
    /// The open commit's details, once asked for.
    pub(super) info: Option<CommitInfo>,
    /// The open commit's message is shown whole.
    pub(super) body_expanded: bool,
    /// The open commit's "Changes" fact, once its files are loaded.
    pub(super) stat: Option<Rc<Fact>>,
    /// Files of the expanded commit, once loaded.
    pub(super) files: Option<Rc<Vec<FileChange>>>,
    /// Which of them are in the working tree now, and the icon that opens one in the editor.
    present: Option<Rc<HashSet<String>>>,
    open: Option<OpenTarget>,
    /// Why they could not be listed.
    files_error: Option<SharedString>,
    error: Option<SharedString>,
    /// The repository, which a submodule's folder is under.
    root: Option<PathBuf>,
    open_path: Option<String>,
    pub(super) code_font: SharedString,
    pub(super) now: i64,
    /// The panel's width, which file names are fitted to.
    pub(super) width: f32,
    picker: PickerChip<History>,
    /// Changes, which a commit's file opens its diff in.
    changes: Option<Entity<Changes>>,
    pub(super) session: i64,
    pub(super) this: WeakEntity<History>,
}

impl HistoryView {
    /// Whether a row has to cut `subject`: it is measured in the panel's font, but only when its length
    /// leaves it in doubt (a few characters never are, many always are), so a frame measures a handful of
    /// subjects at most.
    fn cuts(&self, subject: &str, window: &Window, cx: &App) -> bool {
        let room = self.width - 2.0 * style::CARD_INSET - style::ITEM_LEAD_W - 8.0;
        let chars = subject.chars().count() as f32;
        if chars * 4.0 < room {
            return false;
        }
        if chars * 12.0 > room {
            return true;
        }
        measure::text_width(subject, measure::BODY_REMS, FontWeight::NORMAL, window, cx) > room
    }

    fn render_row(&self, row: &HistoryRow, ui: UiColors, window: &mut Window, cx: &App) -> AnyElement {
        match row {
            HistoryRow::Head => style::section_card(
                "history-band",
                ui,
                style::BandParts {
                    kind: style::BandKind::Section,
                    section: style::Section::Empty,
                    title: "History".into(),
                    detail: None,
                    count: match self.commits.len() {
                        0 => None,
                        n => Some(count_grouped(n).into()),
                    },
                    waiting: None,
                    add: None,
                    on_toggle: None,
                    drop_target: false,
                    sticky: false,
                    joined: false,
                },
                vec![self.picker.render(ui, cx)],
            )
            .into_any_element(),
            // The branch is gone (or git can't read its log): git's words where the list would be.
            HistoryRow::Error => div()
                .px(px(style::CARD_INSET))
                .py_2()
                .text_caption()
                .text_color(ui.danger_text)
                .child(self.error.clone().unwrap_or_default())
                .into_any_element(),
            HistoryRow::Commit(ix) => {
                let ix = *ix;
                let Some(commit) = self.commits.get(ix) else { return div().into_any_element() };
                let (this, session) = (self.this.clone(), self.session);
                // What a click, a right-click and a tooltip need of the commit is looked up when they
                // happen, from the log kept here: a frame copies no hash for each row.
                let (commits, menu_commits, tip_commits) =
                    (self.commits.clone(), self.commits.clone(), self.commits.clone());
                let menu_this = this.clone();
                let row = style::commit_row(
                    SharedString::from(format!("commit-{}", commit.sha)),
                    ui,
                    style::CommitRow {
                        subject: commit.subject.clone().into(),
                        sha: commit.short.clone().into(),
                        author: commit.author.clone().into(),
                        age: age(self.now - commit.time).into(),
                        code_font: self.code_font.clone(),
                        expanded: self.expanded.as_deref() == Some(commit.sha.as_str()),
                        merge: commit.merge,
                    },
                )
                .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                    cx.stop_propagation();
                    let Some(commit) = menu_commits.get(ix) else { return };
                    platform::show_menu(
                        commit_menu(&menu_this, session, &commit.sha),
                        event.position,
                        window,
                        cx,
                    );
                })
                .on_click(move |_, _, cx| {
                    let Some(commit) = commits.get(ix) else { return };
                    let sha = commit.sha.clone();
                    this.update(cx, |this, cx| this.toggle_commit(session, sha, cx)).ok();
                });
                // The whole subject where the row had to cut it.
                if !self.cuts(&commit.subject, window, cx) {
                    return row.into_any_element();
                }
                row.tooltip(move |window, cx| {
                    let subject = tip_commits
                        .get(ix)
                        .map(|c| SharedString::from(c.subject.clone()))
                        .unwrap_or_default();
                    Tooltip::element(move |_, _| div().max_w(px(style::TOOLTIP_W)).child(subject.clone()))
                        .build(window, cx)
                })
                .into_any_element()
            }
            HistoryRow::Details(_) => self.render_details(ui, window, cx),
            HistoryRow::Note(text) => files_line(ui.muted).child(*text).into_any_element(),
            HistoryRow::FilesError => files_line(ui.danger_text)
                .child(self.files_error.clone().unwrap_or_default())
                .into_any_element(),
            HistoryRow::File(ix) => {
                let commit = self.expanded_commit.clone();
                let file = self.files.as_ref().and_then(|files| files.get(*ix)).cloned();
                let (Some(commit), Some(file)) = (commit, file) else { return div().into_any_element() };
                let (this, clicked) = (self.changes.clone(), file.clone());
                let (history, session) = (self.this.clone(), self.session);
                let place = Placement {
                    in_tree: false,
                    depth: 1,
                    selected: self.open_path.as_deref() == Some(file.path.as_str()),
                    width: self.width,
                };
                let row = match (&file.submodule, &self.root) {
                    (Some(_), Some(root)) => {
                        submodule_row(&file, place, root, open_terminal_by_event(self.this.clone()), ui, cx)
                    }
                    _ => {
                        let row = change_row(&file, place, ui, window, cx);
                        // The file as it is now: none for one that is gone from the working tree.
                        match (&self.open, &self.present) {
                            (Some(open), Some(present)) if present.contains(&file.path) => {
                                open.file(&file.path).on_row(row, ui, &format!("history-{}", file.path))
                            }
                            _ => row,
                        }
                    }
                };
                row.on_click(move |_, _, cx| {
                    let Some(changes) = this.clone() else { return };
                    history
                        .update(cx, |history, cx| {
                            history.open_commit_file(session, commit.clone(), clicked.clone(), &changes, cx)
                        })
                        .ok();
                })
                .into_any_element()
            }
            HistoryRow::Spacer => div().h(px(16.0)).into_any_element(),
        }
    }
}

/// A quiet line under the expanded commit, where its files go ("Loading files…").
#[track_caller]
fn files_line(color: Hsla) -> Div {
    div()
        .flex()
        .items_center()
        .min_h(px(style::ROW_H))
        .pl(px(FILES_X))
        .pr(px(style::CARD_INSET))
        .text_caption()
        .text_color(color)
}
