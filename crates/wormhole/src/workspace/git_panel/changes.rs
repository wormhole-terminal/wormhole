//! The Changes tab: git status of the session's repo (optionally compared with
//! another branch), as a list or a folder tree.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::diff::Observed;
use super::file_line::{Placement, change_row};
use super::job::{Job, Ticket, Wanted};
use super::picker::{Picker, PickerChip};
use super::problem::{self, GitTab};
use crate::settings::UiColors;
use crate::settings::{settings_in, ui_in};
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
    /// What the files were (as `signature` says) when the Changes tab was last on screen, so the
    /// strip's amber dot can say something changed since.
    seen: Option<u64>,
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
    /// A fingerprint of the changed files and their line counts; `None` while there are none.
    fn signature(&self) -> Option<u64> {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let files = &self.status.as_ref()?.files;
        if files.is_empty() {
            return None;
        }
        let mut hasher = DefaultHasher::new();
        for file in files {
            (&file.path, file.kind.letter(), file.added, file.removed).hash(&mut hasher);
        }
        Some(hasher.finish())
    }

    /// Whether the changed files are not what the person last saw in the Changes tab: the
    /// amber dot on its tab in the strip.
    pub(crate) fn has_unseen_changes(&self) -> bool {
        self.signature().is_some_and(|now| self.seen != Some(now))
    }

    /// The Changes tab is on screen: what is in it now has been seen. Returns whether the dot
    /// went out, so the strip can be redrawn.
    pub(crate) fn mark_seen(&mut self) -> bool {
        let was = self.has_unseen_changes();
        self.seen = self.signature();
        was
    }

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
        let s = settings_in(&self.profile, cx);
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
        let ui = ui_in(&self.profile, cx);
        let session = self.active_session();
        let panel = div().flex().flex_col().flex_1().min_h_0();

        let Some(status) = &session.git.status else {
            if let Some(error) = &session.git.error {
                return panel
                    .child(self.panel_card_glass(
                        style::panel_card(ui, self.render_git_problem(GitTab::Changes, error, cx)),
                        0,
                        cx,
                    ))
                    .into_any_element();
            }
            // Not a repository: nothing to compare or refresh.
            let text = if session.git.loading() || session.git.cwd.is_none() {
                "Reading git status…"
            } else {
                "This terminal's folder isn't inside a git repository."
            };
            let card = self.panel_card_glass(style::panel_card(ui, style::empty_note(ui, text)), 0, cx);
            return panel.child(card).into_any_element();
        };

        let list = &session.git.list;
        let files = list.built.as_ref().map(|b| b.files.clone()).unwrap_or_default();
        let view = Rc::new(ChangesView {
            files,
            open_path: self.diff_view.as_ref().map(|d| d.file().path.clone()),
            width: self.columns().panel,
            // The compare picker and refresh button sit in the first card's head, where the
            // panel's header row used to be.
            compare: self.picker_chip(Picker::Compare, "vs", status.compare.as_deref(), cx),
            this: cx.weak_entity(),
        });

        let panel = panel
            .children(status.compare_missing.as_deref().map(|branch| problem::compare_missing(ui, branch)))
            .children(session.git.error.as_ref().map(|error| problem::refresh_failed(ui, error)));
        if status.files.is_empty() {
            // The head stays, so the compare picker is still in reach.
            let head = view.render_band(Group::All, 0, false, true, ui, cx);
            let card = style::panel_card(
                ui,
                div()
                    .flex()
                    .flex_col()
                    .child(head)
                    .child(style::empty_note(ui, "No changes. Working tree is clean.")),
            );
            return panel.child(self.panel_card_glass(card, 0, cx)).into_any_element();
        }

        let in_list = view;
        let profile = self.profile.clone();
        panel
            .child(list.rows.element_carded(
                self.panel_ring(window, ui),
                ui,
                self.panel_backing(cx),
                ChangeRow::part,
                move |row, window, cx| in_list.render_row(row, ui_in(&profile, cx), window, cx),
            ))
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
    compare: PickerChip,
    this: WeakEntity<Workspace>,
}

impl ChangesView {
    /// One visible row of the Changes list.
    fn render_row(&self, row: &ChangeRow, ui: UiColors, window: &mut Window, cx: &App) -> AnyElement {
        match row {
            // The first section's head carries the picker and refresh.
            ChangeRow::Band { group, count, folded, first } => {
                self.render_band(*group, *count, *folded, *first, ui, cx)
            }
            ChangeRow::FoldNote(text) => div()
                .px(px(style::CARD_INSET))
                .py_2()
                .text_caption()
                .text_color(ui.muted)
                .child(text.clone())
                .into_any_element(),
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

    /// A section's card head: its title and file count, a click folds it. No +: nothing is added
    /// here. With `controls`, the compare picker and the refresh button come before the count.
    fn render_band(
        &self,
        group: Group,
        count: usize,
        folded: bool,
        controls: bool,
        ui: UiColors,
        cx: &App,
    ) -> AnyElement {
        let this = self.this.clone();
        let refresh = self.this.clone();
        let trailing = if controls {
            vec![
                self.compare.render(ui, cx),
                style::icon_button("git-refresh", ui, IconName::RefreshCw, "Refresh")
                    .in_band()
                    .on_click(move |_, _, cx| {
                        refresh
                            .update(cx, |this, cx| {
                                let ix = this.active;
                                this.refresh_git(ix, cx);
                            })
                            .ok();
                    })
                    .into_any_element(),
            ]
        } else {
            Vec::new()
        };
        style::section_card(
            SharedString::from(format!("changes-band-{}", group.tag())),
            ui,
            style::BandParts {
                kind: style::BandKind::Section,
                section: style::Section::of(count, folded),
                title: group.title().into(),
                detail: None,
                count: (count > 0).then(|| count_grouped(count).into()),
                waiting: None,
                add: None,
                on_toggle: Some(Box::new(move |_, _, cx| {
                    this.update(cx, |this, cx| this.toggle_section(group, count, cx)).ok();
                })),
                drop_target: false,
                sticky: false,
                joined: false,
            },
            trailing,
        )
        .aria_label(spoken::section_band(group.title(), count, spoken::Counted::Files))
        .into_any_element()
    }
}

#[cfg(test)]
mod tests;
