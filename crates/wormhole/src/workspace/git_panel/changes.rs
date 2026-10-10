//! The Changes tab: git status of the session's repo (optionally compared with
//! another branch), as a list or a folder tree.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::diff::Observed;
use super::file_line::{Placement, change_row};
use super::job::{Job, Ticket, Wanted};
use super::picker::Picker;
use super::problem::{self, GitTab};
use crate::settings::{UiColors, settings, ui};
use crate::style::{self, TextRole};
use crate::workspace::Workspace;
use crate::workspace::focus::Walkable;
use crate::workspace::panel_list::Rows;
use wormhole_core::focus::list::Item;
use wormhole_core::git::{self, FileChange, GitError, RepoStatus};
use wormhole_core::limits::count_grouped;
use wormhole_core::panel as rules;
use wormhole_core::spoken;

mod rows;

#[cfg(test)]
use rows::build_tree;
pub(super) use rows::display_order;
use rows::{ChangeRow, Folded, Group, build_rows, collapse_key, file_name, untracked_apart};

/// What a status refresh is for: the folder, and the branch compared with.
type StatusKey = (PathBuf, Option<String>);

/// Git state kept per session.
#[derive(Default)]
pub struct GitState {
    pub status: Option<RepoStatus>,
    /// The cwd the status was computed for.
    pub cwd: Option<PathBuf>,
    /// Why the last refresh failed. When git failed, the status stays the last
    /// good one (while the folder is the same), so a hiccup does not blank the
    /// list; a missing git clears it.
    error: Option<GitError>,
    /// The refresh in flight.
    job: Job<StatusKey>,
    /// The branch list the open picker waits for (see `picker.rs`).
    pub(super) branches: Job<(Picker, PathBuf)>,
    /// Sections the user folded (`true`) or unfolded; the others follow the size rule
    /// (`wormhole_core::panel::section_folded`).
    folds: HashMap<Group, bool>,
    list: ChangesList,
}

/// What became of a refresh's answer.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Refreshed {
    /// Not the answer wanted any more (a newer request superseded it, or the
    /// session moved to another folder or branch while git ran): dropped.
    Stale,
    Applied {
        changed: bool,
    },
}

impl GitState {
    pub fn loading(&self) -> bool {
        self.job.is_pending()
    }

    /// git is missing: nothing polls it again until Retry (or the window coming to the
    /// front), since on a Mac without the Command Line Tools every run of the stub asks
    /// to install them.
    pub(crate) fn waits_for_retry(&self) -> bool {
        self.error == Some(GitError::GitMissing)
    }

    /// Start a refresh for `key`. `None` when that same refresh is already
    /// running; a different one (another branch compared, another folder)
    /// supersedes it, and the older answer is dropped when it lands.
    pub(super) fn begin_refresh(&mut self, key: StatusKey) -> Option<Wanted> {
        self.job.begin(key)
    }

    /// Take a refresh's answer. `now` is what the session shows at this moment.
    pub(super) fn finish_refresh(
        &mut self,
        ticket: Ticket,
        now: Option<&StatusKey>,
        result: anyhow::Result<Option<RepoStatus>>,
    ) -> Refreshed {
        let Some(key) = self.job.finish(ticket) else { return Refreshed::Stale };
        if now != Some(&key) {
            // The next poll asks again for what is wanted now.
            return Refreshed::Stale;
        }
        let same_folder = self.cwd.as_ref() == Some(&key.0);
        self.cwd = Some(key.0);
        let changed = match result.map_err(|err| (GitError::of(&err), err)) {
            Ok(status) => {
                let changed = self.status != status || self.error.is_some();
                self.error = None;
                self.status = status;
                changed
            }
            // The folder stopped being a repository after git found one: as if it never was.
            Err((GitError::NotARepo, _)) => {
                let changed = self.status.is_some() || self.error.is_some();
                (self.status, self.error) = (None, None);
                changed
            }
            Err((error, err)) => {
                if self.error.as_ref() != Some(&error) {
                    // In full, once: the panel shows git's first line.
                    tracing::warn!(cwd = ?self.cwd, "git status failed: {err:#}");
                }
                // Another folder's list under this folder's name would mislead, and without git
                // the list is not coming back by itself.
                let keep = same_folder && error != GitError::GitMissing;
                let changed = self.error.as_ref() != Some(&error) || (!keep && self.status.is_some());
                if !keep {
                    self.status = None;
                }
                self.error = Some(error);
                changed
            }
        };
        Refreshed::Applied { changed }
    }
}

/// What the rows were built from. They are rebuilt only when this changes,
/// not on every scroll tick.
struct Built {
    files: Rc<Vec<FileChange>>,
    tree: bool,
    group_untracked: bool,
    collapsed: HashSet<String>,
    folds: HashMap<Group, bool>,
}

#[derive(Default)]
struct ChangesList {
    built: Option<Built>,
    rows: Rows<ChangeRow>,
}

impl ChangesList {
    fn sync(
        &mut self,
        status: Option<&RepoStatus>,
        tree: bool,
        group_untracked: bool,
        folded: Folded,
        folds: &HashMap<Group, bool>,
    ) {
        let Some(status) = status else {
            self.built = None;
            self.rows.set(Vec::new());
            return;
        };
        let group_untracked = untracked_apart(status, group_untracked);
        let same_files = self.built.as_ref().is_some_and(|b| b.files[..] == status.files[..]);
        if same_files
            && self.built.as_ref().is_some_and(|b| {
                b.tree == tree
                    && b.group_untracked == group_untracked
                    && b.collapsed == *folded.set
                    && b.folds == *folds
            })
        {
            return;
        }
        let files = match &self.built {
            Some(b) if same_files => b.files.clone(),
            _ => Rc::new(status.files.clone()),
        };
        self.rows.set(build_rows(&files, tree, group_untracked, folded, folds));
        self.built =
            Some(Built { files, tree, group_untracked, collapsed: folded.set.clone(), folds: folds.clone() });
    }
}

impl Workspace {
    // ------------------------------------------------------------ status

    /// Refresh git status for a session in the background.
    pub(crate) fn refresh_git(&mut self, ix: usize, cx: &mut Context<Self>) {
        // An open diff's file is looked at in the same background hop, so a diff
        // an agent made stale is noticed within one poll (see `refresh_open_diff`).
        let watched =
            if ix == self.active { self.diff_view.as_ref().and_then(|v| v.watched_path()) } else { None };
        let Some(session) = self.sessions.get_mut(ix) else { return };
        let Some(cwd) = session.terminal().read(cx).cwd.clone() else { return };
        let compare = session.compare.clone();
        let Some(wanted) = session.git.begin_refresh((cwd.clone(), compare.clone())) else { return };
        let (id, ticket) = (session.id, wanted.ticket());
        let task = cx.background_executor().spawn(async move {
            let observed = watched.map(Observed::of);
            // A compare branch that was deleted falls back to HEAD there, and says so.
            (git::status(&cwd, compare.as_deref()), observed)
        });
        cx.spawn(async move |this, cx| {
            let (status, observed) = task.await;
            let _ = this.update(cx, |this, cx| this.finish_git_refresh(id, ticket, status, observed, cx));
        })
        .detach();
    }

    fn finish_git_refresh(
        &mut self,
        id: i64,
        ticket: Ticket,
        status: anyhow::Result<Option<RepoStatus>>,
        observed: Option<Observed>,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.sessions.iter().position(|s| s.id == id) else { return };
        let session = &mut self.sessions[ix];
        let now = session.terminal().read(cx).cwd.clone().map(|cwd| (cwd, session.compare.clone()));
        match session.git.finish_refresh(ticket, now.as_ref(), status) {
            Refreshed::Stale => {}
            Refreshed::Applied { changed } => {
                if changed {
                    cx.notify();
                }
                if ix == self.active {
                    self.refresh_open_diff(observed, cx);
                }
            }
        }
    }

    /// A click on a section's band: fold it, or unfold it (one folded for its size too).
    fn toggle_section(&mut self, group: Group, count: usize, cx: &mut Context<Self>) {
        let git = &mut self.sessions[self.active].git;
        let folded = rules::section_folded(count, git.folds.get(&group).copied());
        git.folds.insert(group, !folded);
        cx.notify();
    }

    fn toggle_dir(&mut self, group: Group, path: String, cx: &mut Context<Self>) {
        let key = collapse_key(self.active_session().id, group, &path);
        if !self.collapsed_dirs.remove(&key) {
            self.collapsed_dirs.insert(key);
        }
        cx.notify();
    }

    // ------------------------------------------------------------ changes tab

    /// Refresh the list's rows from git status and settings. Cheap when nothing
    /// changed, so it runs on every render (including every scroll tick).
    pub(crate) fn sync_changes_list(&mut self, cx: &App) {
        let s = settings(cx);
        let (tree, group_untracked) = (s.changes_tree, s.group_untracked);
        let session = &mut self.sessions[self.active];
        let folded = Folded { set: &self.collapsed_dirs, session: session.id };
        let git = &mut session.git;
        git.list.sync(git.status.as_ref(), tree, group_untracked, folded, &git.folds);
    }

    // ------------------------------------------------------------ keys

    /// The Changes rows the keys can be on: each section's band (a ⇥ stop), its folders and files.
    pub(in crate::workspace) fn changes_walkable(&self) -> Vec<Walkable> {
        let rows = self.active_session().git.list.rows.items();
        (rows.iter().enumerate())
            .filter_map(|(row, line)| {
                let (level, fold, band) = match line {
                    ChangeRow::Band { count, folded, .. } => (0, (*count > 0).then_some(!folded), true),
                    ChangeRow::Dir { depth, collapsed, .. } => (1 + *depth as u8, Some(!collapsed), false),
                    ChangeRow::File { depth, .. } => (1 + *depth as u8, None, false),
                    ChangeRow::FoldNote(_) | ChangeRow::Spacer => return None,
                };
                Some(Walkable { row, item: Item { level, fold }, band })
            })
            .collect()
    }

    /// → / ← on a band folds or opens its section, on a folder the folder.
    pub(in crate::workspace) fn changes_fold(&mut self, row: usize, cx: &mut Context<Self>) {
        match self.active_session().git.list.rows.items().get(row).cloned() {
            Some(ChangeRow::Band { group, count, .. }) => self.toggle_section(group, count, cx),
            Some(ChangeRow::Dir { group, path, .. }) => self.toggle_dir(group, path, cx),
            _ => {}
        }
    }

    /// A press on a file opens its diff, on a band or folder folds it (`following`: only a
    /// file's). `focus/panel.rs` sends ↩ here, and Space only where it folds. The list keeps
    /// the keyboard.
    pub(in crate::workspace) fn changes_press(
        &mut self,
        row: usize,
        following: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let list = &self.active_session().git.list;
        match list.rows.items().get(row) {
            Some(ChangeRow::File { ix, .. }) => {
                if let Some(file) = list.built.as_ref().and_then(|b| b.files.get(*ix)).cloned() {
                    self.open_change(file, cx);
                }
            }
            Some(ChangeRow::Band { .. } | ChangeRow::Dir { .. }) if !following => self.changes_fold(row, cx),
            _ => {}
        }
        true
    }

    pub(in crate::workspace) fn changes_reveal(&self, row: usize) {
        self.active_session().git.list.rows.reveal(row);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_changes_list(&self) -> ListState {
        self.active_session().git.list.rows.state()
    }

    pub(crate) fn render_changes_tab(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let session = self.active_session();
        let panel = div().flex().flex_col().flex_1().min_h_0().pt(px(style::PANEL_LIST_TOP));

        let Some(status) = &session.git.status else {
            if let Some(error) = &session.git.error {
                return panel.child(self.render_git_problem(GitTab::Changes, error, cx)).into_any_element();
            }
            // Not a repository: no header, nothing to compare or refresh.
            let text = if session.git.loading() || session.git.cwd.is_none() {
                "Reading git status…"
            } else {
                "This terminal's folder isn't inside a git repository."
            };
            return panel.child(style::empty_note(ui, text).px_4()).into_any_element();
        };

        // The branch chip · the repository, the compare picker and refresh, ending on x 344.
        let header = div()
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .h(px(style::TITLE_ROW_H))
            .pl(px(style::PANEL_INSET))
            .pr(px(style::HEADER_PAD_END))
            .child(style::branch_chip(ui, status.branch.as_deref(), None))
            .child(div().flex_none().text_caption().text_color(ui.faint).child("·"))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_caption()
                    .text_color(ui.muted)
                    .child(short_root(&status.root)),
            )
            .child(div().flex_1())
            .child(self.render_picker_chip(Picker::Compare, "vs", status.compare.as_deref(), cx))
            .child(
                style::icon_button("git-refresh", ui, IconName::RefreshCw, "Refresh")
                    .in_title_row()
                    .on_click(cx.listener(|this, _, _, cx| {
                        let ix = this.active;
                        this.refresh_git(ix, cx);
                    })),
            );

        let panel = panel
            .child(header)
            .children(status.compare_missing.as_deref().map(|branch| problem::compare_missing(ui, branch)))
            .children(session.git.error.as_ref().map(|error| problem::refresh_failed(ui, error)));
        if status.files.is_empty() {
            // The header stays, so the compare picker is still in reach.
            return panel
                .child(style::empty_note(ui, "No changes. Working tree is clean.").px_4())
                .into_any_element();
        }

        let list = &session.git.list;
        let files = list.built.as_ref().map(|b| b.files.clone()).unwrap_or_default();
        let view = Rc::new(ChangesView {
            files,
            open_path: self.diff_view.as_ref().map(|d| d.file().path.clone()),
            width: self.columns().panel,
            this: cx.weak_entity(),
        });
        let pinned =
            list.rows.pinned(px(style::BAND_H), ChangeRow::is_band).map(|(row, push)| (row.clone(), push));
        let in_list = view.clone();
        panel
            .child(
                // A band whose section scrolls under the top stays pinned there.
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .child(list.rows.element(self.panel_ring(window, ui), move |row, window, cx| {
                        in_list.render_row(row, false, crate::settings::ui(cx), window, cx)
                    }))
                    .children(pinned.map(|(row, push)| {
                        div().absolute().top(push).left_0().right_0().child(view.render_band(&row, true, ui))
                    })),
            )
            .into_any_element()
    }
}

/// What the Changes list's rows draw from, captured for the render callback.
struct ChangesView {
    files: Rc<Vec<FileChange>>,
    /// The file whose diff is open: its row is the selected one.
    open_path: Option<String>,
    /// The panel's width, which file names are fitted to.
    width: f32,
    this: WeakEntity<Workspace>,
}

impl ChangesView {
    /// One visible row of the Changes list; `pinned` is a band's copy over the top of the list.
    fn render_row(
        &self,
        row: &ChangeRow,
        pinned: bool,
        ui: UiColors,
        window: &mut Window,
        cx: &App,
    ) -> AnyElement {
        match row {
            ChangeRow::Band { .. } => self.render_band(row, pinned, ui),
            ChangeRow::FoldNote(text) => {
                div().px_4().py_2().text_caption().text_color(ui.muted).child(text.clone()).into_any_element()
            }
            ChangeRow::Dir { group, path, label, depth, collapsed } => {
                let (this, group, key) = (self.this.clone(), *group, path.clone());
                let icon = crate::icons::folder(file_name(label), !collapsed, cx)
                    .unwrap_or_else(|| style::icon(IconName::Folder).into_any_element());
                style::folder_row(
                    SharedString::from(format!("dir-{}-{path}", group.tag())),
                    ui,
                    *depth,
                    icon,
                    label.clone(),
                    *collapsed,
                )
                .on_click(move |_, _, cx| {
                    this.update(cx, |this, cx| this.toggle_dir(group, key.clone(), cx)).ok();
                })
                .into_any_element()
            }
            ChangeRow::File { ix, depth, in_tree } => {
                let Some(file) = self.files.get(*ix) else { return div().into_any_element() };
                let (this, clicked) = (self.this.clone(), file.clone());
                let place = Placement {
                    in_tree: *in_tree,
                    depth: *depth,
                    selected: self.open_path.as_deref() == Some(file.path.as_str()),
                    width: self.width,
                };
                change_row(file, place, ui, window, cx)
                    .on_click(move |_, _, cx| {
                        this.update(cx, |this, cx| this.open_change(clicked.clone(), cx)).ok();
                    })
                    .into_any_element()
            }
            ChangeRow::Spacer => div().h(px(16.0)).into_any_element(),
        }
    }

    /// A section's band: its title and file count, a click folds it. No +: nothing is added here.
    fn render_band(&self, row: &ChangeRow, pinned: bool, ui: UiColors) -> AnyElement {
        let ChangeRow::Band { group, count, folded, joined } = *row else { return div().into_any_element() };
        let this = self.this.clone();
        let id = SharedString::from(format!(
            "changes-band-{}{}",
            group.tag(),
            if pinned { "-pinned" } else { "" }
        ));
        style::band(
            id,
            ui,
            style::BandParts {
                kind: style::BandKind::Section,
                section: style::Section::of(count, folded),
                title: group.title().into(),
                detail: None,
                count: Some(count_grouped(count).into()),
                add: None,
                on_toggle: Some(Box::new(move |_, _, cx| {
                    this.update(cx, |this, cx| this.toggle_section(group, count, cx)).ok();
                })),
                drop_target: false,
                sticky: pinned,
                joined: joined && !pinned,
            },
        )
        .aria_label(spoken::section_band(group.title(), count, spoken::Counted::Files))
        .into_any_element()
    }
}

fn short_root(root: &Path) -> String {
    root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| root.display().to_string())
}

#[cfg(test)]
mod tests;
