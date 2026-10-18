//! The History tab: commit log per branch; expand a commit to see its files
//! and open their diffs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::file_line::{Placement, change_row};
use super::job::{Job, Ticket, Wanted};
use super::picker::{Picker, PickerChip};
use super::problem::GitTab;
use crate::settings::UiColors;
use crate::settings::ui_in;
use crate::style::{self, TextRole};
use crate::workspace::Workspace;
use crate::workspace::panel_list::{CardPart, Rows};
use wormhole_core::git::{self, Commit, FileChange, GitError, LOG_LIMIT};
use wormhole_core::limits::count_grouped;
use wormhole_core::panel::age;

/// What a history refresh is for: the folder, and the branch whose log is shown.
type HistoryKey = (PathBuf, Option<String>);

/// What a refresh found: the repository root, and its log (or why git could not
/// list it); `Ok(None)` when the folder is not in a repository, `Err` when git
/// could not tell (it is missing, or failed).
type Fetched = Result<Option<(PathBuf, Result<Vec<Commit>, String>)>, GitError>;

/// The files of one commit, as far as they are known.
pub enum CommitFiles {
    /// Being fetched.
    Loading,
    Loaded(Rc<Vec<FileChange>>),
    /// Git failed. Shown while the commit stays open; not kept, so opening it again tries again.
    Failed(SharedString),
}

/// How many commits' files are kept; the open one and those in flight always are.
const FILES_KEPT: usize = 64;

/// Per-session commit history.
#[derive(Default)]
pub struct HistoryState {
    /// Branch whose log is shown; `None` = HEAD.
    pub rev: Option<String>,
    pub commits: Rc<Vec<Commit>>,
    /// The refresh in flight.
    job: Job<HistoryKey>,
    /// What the shown answer is for. Until git has answered for the folder and branch on
    /// screen, an empty history is still being read, not "not a repository".
    answered: Option<HistoryKey>,
    /// Why the log of the branch could not be read (it was deleted, say).
    pub error: Option<SharedString>,
    /// Why git could not read the folder at all: drawn instead of the list.
    problem: Option<GitError>,
    /// Commit expanded to show its files.
    pub expanded: Option<String>,
    files: HashMap<String, CommitFiles>,
    pub root: Option<PathBuf>,
    list: Rows<HistoryRow>,
}

/// One line of the History list: a commit, or what sits under the expanded one.
#[derive(Clone, PartialEq)]
enum HistoryRow {
    /// The card's head: "History", how many commits, and the branch picker.
    Head,
    Error,
    Commit(usize),
    /// "Loading files…" / "No file changes." under the expanded commit.
    Note(&'static str),
    /// Why the expanded commit's files could not be listed.
    FilesError,
    /// Index into the expanded commit's files.
    File(usize),
    /// Bottom padding, so the last row can scroll clear of the panel edge.
    Spacer,
}

impl HistoryRow {
    /// The commits are one card; the spacer under it is outside.
    fn part(&self) -> CardPart {
        match self {
            Self::Head => CardPart::Head,
            Self::Spacer => CardPart::Outside,
            _ => CardPart::Body,
        }
    }
}

impl HistoryState {
    /// An empty history showing branch `rev` (`None` = HEAD).
    pub fn for_rev(rev: Option<String>) -> Self {
        Self { rev, ..Self::default() }
    }

    #[cfg(any(test, feature = "devctl"))]
    pub fn loading(&self) -> bool {
        self.job.is_pending()
    }

    /// Whether git has answered for `cwd` and the branch shown (whatever it said).
    pub(crate) fn answered_for(&self, cwd: &Path) -> bool {
        self.answered.as_ref().is_some_and(|(dir, rev)| dir == cwd && *rev == self.rev)
    }

    /// git is missing: like the Changes tab, nothing polls it until Retry.
    pub(crate) fn waits_for_retry(&self) -> bool {
        self.problem == Some(GitError::GitMissing)
    }

    /// Start a refresh for `key`. `None` when that same refresh is already
    /// running; another branch or folder supersedes it and the older answer is
    /// dropped when it lands.
    fn begin_refresh(&mut self, key: HistoryKey) -> Option<Wanted> {
        self.job.begin(key)
    }

    /// Take a refresh's answer; `now` is what the session shows at this moment.
    /// Returns whether the list changed.
    fn finish_refresh(&mut self, ticket: Ticket, now: Option<&HistoryKey>, result: Fetched) -> bool {
        let Some(key) = self.job.finish(ticket) else { return false };
        if now != Some(&key) {
            // The next poll asks again for what is wanted now.
            return false;
        }
        let had_problem = self.problem.take().is_some();
        let first_answer = self.answered.as_ref() != Some(&key);
        self.answered = Some(key);
        match result {
            Ok(Some((root, Ok(commits)))) => {
                self.error = None;
                self.root = Some(root);
                if *self.commits == commits && !had_problem && !first_answer {
                    return false;
                }
                self.commits = Rc::new(commits);
                true
            }
            Ok(Some((root, Err(err)))) => {
                self.root = Some(root);
                self.error = Some(err.into());
                true
            }
            Ok(None) => {
                self.root = None;
                self.commits = Rc::default();
                true
            }
            Err(problem) => {
                (self.root, self.commits, self.error) = (None, Rc::default(), None);
                self.problem = Some(problem);
                true
            }
        }
    }

    /// Expand `sha`, or collapse it when it is the open one. Returns the commit
    /// whose files have to be fetched (once: one already on its way is awaited).
    fn toggle(&mut self, sha: &str) -> Option<(PathBuf, Commit)> {
        let open = self.expanded.as_deref() == Some(sha);
        // A failure is not kept: closing or opening again is a retry.
        if matches!(self.files.get(sha), Some(CommitFiles::Failed(_))) {
            self.files.remove(sha);
        }
        if open {
            self.expanded = None;
            return None;
        }
        self.expanded = Some(sha.to_string());
        if self.files.contains_key(sha) {
            return None;
        }
        let root = self.root.clone()?;
        let commit = self.commits.iter().find(|c| c.sha == sha)?.clone();
        self.files.insert(sha.to_string(), CommitFiles::Loading);
        Some((root, commit))
    }

    /// Open `commit` of the repository at `root` (the command palette's commit, which may not be
    /// listed yet): on the branch shown when `on_branch` says the commit is on it, else on HEAD.
    /// Returns the commit whose files have to be fetched.
    fn open(&mut self, root: PathBuf, commit: Commit, on_branch: bool) -> Option<(PathBuf, Commit)> {
        if !on_branch {
            self.rev = None;
        }
        self.expanded = Some(commit.sha.clone());
        if matches!(self.files.get(&commit.sha), Some(CommitFiles::Failed(_))) {
            self.files.remove(&commit.sha);
        }
        if self.files.contains_key(&commit.sha) {
            return None;
        }
        self.files.insert(commit.sha.clone(), CommitFiles::Loading);
        Some((root, commit))
    }

    /// The files of `sha` arrived (or git failed to list them).
    fn files_loaded(&mut self, sha: &str, result: Result<Vec<FileChange>, String>) {
        match result {
            Ok(files) => {
                self.files.insert(sha.to_string(), CommitFiles::Loaded(Rc::new(files)));
                if self.files.len() > FILES_KEPT {
                    let expanded = self.expanded.clone();
                    self.files.retain(|key, files| {
                        Some(key.as_str()) == expanded.as_deref()
                            || key == sha
                            || matches!(files, CommitFiles::Loading)
                    });
                }
            }
            // Only for a commit that is still waiting for it, and still open.
            Err(err) if matches!(self.files.get(sha), Some(CommitFiles::Loading)) => {
                if self.expanded.as_deref() == Some(sha) {
                    self.files.insert(sha.to_string(), CommitFiles::Failed(err.into()));
                } else {
                    self.files.remove(sha);
                }
            }
            Err(_) => {}
        }
    }

    /// The files of `sha`, once loaded.
    fn loaded(&self, sha: &str) -> Option<&Rc<Vec<FileChange>>> {
        match self.files.get(sha) {
            Some(CommitFiles::Loaded(files)) => Some(files),
            _ => None,
        }
    }

    fn rows(&self) -> Vec<HistoryRow> {
        let mut rows = Vec::with_capacity(self.commits.len() + 3);
        rows.push(HistoryRow::Head);
        if self.error.is_some() {
            rows.push(HistoryRow::Error);
        }
        for (ix, commit) in self.commits.iter().enumerate() {
            rows.push(HistoryRow::Commit(ix));
            if self.expanded.as_deref() != Some(commit.sha.as_str()) {
                continue;
            }
            match self.files.get(&commit.sha) {
                None | Some(CommitFiles::Loading) => rows.push(HistoryRow::Note("Loading files…")),
                Some(CommitFiles::Failed(_)) => rows.push(HistoryRow::FilesError),
                Some(CommitFiles::Loaded(files)) if files.is_empty() => {
                    rows.push(HistoryRow::Note("No file changes."))
                }
                Some(CommitFiles::Loaded(files)) => rows.extend((0..files.len()).map(HistoryRow::File)),
            }
        }
        rows.push(HistoryRow::Spacer);
        rows
    }
}

impl Workspace {
    // ------------------------------------------------------------ data

    /// The History tab shows a folder or branch git hasn't answered for yet (a terminal just
    /// opened or switched to): the tick asks right away instead of at the next poll.
    pub(crate) fn history_unanswered(&self, cx: &App) -> bool {
        let session = self.active_session();
        session.terminal().read(cx).cwd.as_deref().is_some_and(|cwd| !session.history.answered_for(cwd))
    }

    pub(crate) fn refresh_history(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get_mut(ix) else { return };
        let Some(cwd) = session.terminal().read(cx).cwd.clone() else { return };
        let rev = session.history.rev.clone();
        let Some(wanted) = session.history.begin_refresh((cwd.clone(), rev.clone())) else { return };
        let (id, ticket) = (session.id, wanted.ticket());
        let task = cx.background_executor().spawn(async move {
            let root = match git::repo_root(&cwd) {
                Ok(Some(root)) => root,
                Ok(None) => return Ok(None),
                Err(err) => {
                    tracing::debug!(?cwd, "git could not read the history: {err:#}");
                    return Err(GitError::of(&err));
                }
            };
            // A branch that was deleted since it was picked gets its own words.
            let rev = rev.as_deref().unwrap_or("HEAD");
            let commits = git::log(&root, rev, LOG_LIMIT).map_err(|err| git::log_failure(&root, rev, &err));
            Ok(Some((root, commits)))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                let Some(session) = this.sessions.iter_mut().find(|s| s.id == id) else { return };
                let now =
                    session.terminal().read(cx).cwd.clone().map(|cwd| (cwd, session.history.rev.clone()));
                if session.history.finish_refresh(ticket, now.as_ref(), result) {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn toggle_commit(&mut self, sha: String, cx: &mut Context<Self>) {
        let session = &mut self.sessions[self.active];
        let id = session.id;
        let fetch = session.history.toggle(&sha);
        cx.notify();
        if let Some((root, commit)) = fetch {
            self.fetch_commit_files(id, root, commit, cx);
        }
    }

    /// The command palette's commit: the History tab of the current terminal with the
    /// commit open, on the branch it shows when the commit is on it (git says, off the UI
    /// thread), else on HEAD.
    pub(crate) fn reveal_commit(&mut self, root: PathBuf, commit: Commit, cx: &mut Context<Self>) {
        let session = self.active_session();
        let id = session.id;
        let Some(rev) = session.history.rev.clone() else {
            return self.open_commit(id, root, commit, true, cx);
        };
        let asked = cx.background_executor().spawn({
            let (root, sha) = (root.clone(), commit.sha.clone());
            async move { git::is_ancestor(&root, &sha, &rev).unwrap_or(false) }
        });
        cx.spawn(async move |this, cx| {
            let on_branch = asked.await;
            let _ = this.update(cx, |this, cx| this.open_commit(id, root, commit, on_branch, cx));
        })
        .detach();
    }

    fn open_commit(
        &mut self,
        id: i64,
        root: PathBuf,
        commit: Commit,
        on_branch: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.sessions.iter_mut().find(|s| s.id == id) else { return };
        let fetch = session.history.open(root, commit, on_branch);
        self.show_tab(crate::workspace::RightTab::History, cx);
        cx.notify();
        if let Some((root, commit)) = fetch {
            self.fetch_commit_files(id, root, commit, cx);
        }
    }

    /// Read the files of `commit` off the UI thread, for session `id`.
    fn fetch_commit_files(&mut self, id: i64, root: PathBuf, commit: Commit, cx: &mut Context<Self>) {
        let sha = commit.sha.clone();
        let task = cx
            .background_executor()
            .spawn(async move { git::commit_files(&root, &commit).map_err(|e| format!("{e:#}")) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(session) = this.sessions.iter_mut().find(|s| s.id == id) {
                    session.history.files_loaded(&sha, result);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn open_commit_file(&mut self, commit: Commit, file: FileChange, cx: &mut Context<Self>) {
        let session = self.active_session();
        let Some(root) = session.history.root.clone() else { return };
        let siblings = session.history.loaded(&commit.sha).cloned().unwrap_or_default();
        let spec = git::commit_spec(&commit);
        self.open_diff(root, file, spec, siblings, Some(commit.short.clone().into()), cx);
    }

    // ------------------------------------------------------------ devctl

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_history(&mut self, cx: &mut Context<Self>) {
        self.show_tab(crate::workspace::RightTab::History, cx);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_toggle_commit(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(sha) = self.active_session().history.commits.get(ix).map(|c| c.sha.clone()) {
            self.toggle_commit(sha, cx);
        }
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_commit_file(&mut self, ix: usize, cx: &mut Context<Self>) {
        let history = &self.active_session().history;
        let Some(sha) = history.expanded.clone() else { return };
        let commit = history.commits.iter().find(|c| c.sha == sha).cloned();
        let file = history.loaded(&sha).and_then(|f| f.get(ix).cloned());
        if let (Some(commit), Some(file)) = (commit, file) {
            self.open_commit_file(commit, file, cx);
        }
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_compare(&mut self, branch: Option<String>, cx: &mut Context<Self>) {
        self.pick_branch(Picker::Compare, branch, cx);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_history_branch(&mut self, branch: Option<String>, cx: &mut Context<Self>) {
        self.pick_branch(Picker::History, branch, cx);
    }

    /// Open the branch select in the Changes ("compare") or History tab,
    /// optionally with `query` typed into its search box.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_picker(&mut self, arg: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (which, query) = arg.split_once(' ').unwrap_or((arg, ""));
        let (tab, picker) = match which {
            "history" => (crate::workspace::RightTab::History, Picker::History),
            _ => (crate::workspace::RightTab::Changes, Picker::Compare),
        };
        self.show_tab(tab, cx);
        if self.picker != Some(picker) {
            self.toggle_picker(picker, window, cx);
        }
        let query = query.to_string();
        self.picker_search.update(cx, |search, cx| search.set_value(query, window, cx));
    }

    // ------------------------------------------------------------ branch picker

    // ------------------------------------------------------------ history tab

    /// Refresh the list's rows from the commits and the expanded commit. Cheap
    /// when nothing changed, so it runs on every render (every scroll tick).
    pub(crate) fn sync_history_list(&mut self) {
        let history = &mut self.sessions[self.active].history;
        let rows = history.rows();
        history.list.set(rows);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_history_list(&self) -> ListState {
        self.active_session().history.list.state()
    }

    pub(crate) fn render_history_tab(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let session = self.active_session();
        let history = &session.history;
        let panel = div().flex().flex_col().flex_1().min_h_0();

        if history.root.is_none() {
            if let Some(problem) = &history.problem {
                return panel
                    .child(style::panel_card(ui, self.render_git_problem(GitTab::History, problem, cx)))
                    .into_any_element();
            }
            let answered =
                session.terminal().read(cx).cwd.as_deref().is_some_and(|cwd| history.answered_for(cwd));
            let text = if answered {
                "This terminal's folder isn't inside a git repository."
            } else {
                "Reading history…"
            };
            return panel.child(style::panel_card(ui, style::empty_note(ui, text))).into_any_element();
        }

        let view = HistoryView {
            commits: history.commits.clone(),
            expanded: history.expanded.clone(),
            expanded_commit: history
                .expanded
                .as_ref()
                .and_then(|sha| history.commits.iter().find(|c| &c.sha == sha).cloned()),
            files: history.expanded.as_ref().and_then(|sha| history.loaded(sha).cloned()),
            files_error: history.expanded.as_ref().and_then(|sha| match history.files.get(sha) {
                Some(CommitFiles::Failed(err)) => Some(err.clone()),
                _ => None,
            }),
            error: history.error.clone(),
            open_path: self.diff_view.as_ref().map(|d| d.file().path.clone()),
            code_font: crate::fonts::code_font_in(&self.profile, cx).0,
            now: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            width: self.columns().panel,
            picker: self.picker_chip(Picker::History, "", history.rev.as_deref(), cx),
            this: cx.weak_entity(),
        };

        let profile = self.profile.clone();
        // A column that fills the panel, so the list gets a bounded height.
        panel
            .child(history.list.with_edges(
                history.list.element_carded(
                    self.panel_ring(window, ui),
                    ui,
                    HistoryRow::part,
                    move |row, window, cx| view.render_row(row, ui_in(&profile, cx), window, cx),
                ),
                ui,
                "history-scrollbar",
                cx,
            ))
            .into_any_element()
    }
}

/// Where a commit's files and the lines about them start: one level in (x 40).
const FILES_X: f32 = style::CARD_INSET + style::INDENT;

/// What the History list's rows draw from, captured for the render callback.
struct HistoryView {
    commits: Rc<Vec<Commit>>,
    expanded: Option<String>,
    expanded_commit: Option<Commit>,
    /// Files of the expanded commit, once loaded.
    files: Option<Rc<Vec<FileChange>>>,
    /// Why they could not be listed.
    files_error: Option<SharedString>,
    error: Option<SharedString>,
    open_path: Option<String>,
    code_font: SharedString,
    now: i64,
    /// The panel's width, which file names are fitted to.
    width: f32,
    picker: PickerChip,
    this: WeakEntity<Workspace>,
}

impl HistoryView {
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
                let Some(commit) = self.commits.get(*ix) else { return div().into_any_element() };
                let (this, sha) = (self.this.clone(), commit.sha.clone());
                style::commit_row(
                    SharedString::from(format!("commit-{}", commit.sha)),
                    ui,
                    style::CommitRow {
                        subject: commit.subject.clone().into(),
                        sha: commit.short.clone().into(),
                        author: commit.author.clone().into(),
                        age: age(self.now - commit.time).into(),
                        code_font: self.code_font.clone(),
                        expanded: self.expanded.as_deref() == Some(commit.sha.as_str()),
                    },
                )
                .on_click(move |_, _, cx| {
                    this.update(cx, |this, cx| this.toggle_commit(sha.clone(), cx)).ok();
                })
                .into_any_element()
            }
            HistoryRow::Note(text) => files_line(ui.muted).child(*text).into_any_element(),
            HistoryRow::FilesError => files_line(ui.danger_text)
                .child(self.files_error.clone().unwrap_or_default())
                .into_any_element(),
            HistoryRow::File(ix) => {
                let commit = self.expanded_commit.clone();
                let file = self.files.as_ref().and_then(|files| files.get(*ix)).cloned();
                let (Some(commit), Some(file)) = (commit, file) else { return div().into_any_element() };
                let (this, clicked) = (self.this.clone(), file.clone());
                let place = Placement {
                    in_tree: false,
                    depth: 1,
                    selected: self.open_path.as_deref() == Some(file.path.as_str()),
                    width: self.width,
                };
                change_row(&file, place, ui, window, cx)
                    .on_click(move |_, _, cx| {
                        this.update(cx, |this, cx| {
                            this.open_commit_file(commit.clone(), clicked.clone(), cx)
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

mod keys;
#[cfg(test)]
mod tests;
