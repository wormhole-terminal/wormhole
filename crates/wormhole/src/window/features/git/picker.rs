//! The branch picker shared by the Changes tab ("compare with") and the
//! History tab (which branch's log).

use std::path::{Path, PathBuf};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::InputState;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::history::HistoryState;
use super::job::Ticket;
use crate::select::{self, Choice, SearchSelect, SelectTrigger};
use crate::settings::{UiColors, ui_in};
use crate::window::Workspace;
use wormhole_core::git;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Picker {
    /// Changes tab: compare the working tree with another branch.
    Compare,
    /// History tab: which branch's log to show.
    History,
}

impl Workspace {
    pub(crate) fn toggle_picker(&mut self, picker: Picker, window: &mut Window, cx: &mut Context<Self>) {
        if self.picker == Some(picker) {
            self.picker = None;
            self.branches.clear();
            cx.notify();
            return;
        }
        self.picker = Some(picker);
        self.branches.clear();
        self.picker_search.update(cx, |search, cx| search.set_value("", window, cx));
        cx.notify();
        let Some(cwd) = self.active_terminal().read(cx).cwd.clone() else { return };
        let session = &mut self.sessions[self.active];
        // The same list already being fetched (closed and reopened quickly) is waited for.
        let Some(wanted) = session.git.branches.begin((picker, cwd.clone())) else { return };
        let (id, ticket) = (session.id, wanted.ticket());
        // The picker sits in a header that is only drawn once git has answered, so a failure
        // here is the panel's to show, not the list's: it just stays empty.
        let task = cx.background_executor().spawn(async move {
            match git::repo_root(&cwd) {
                Ok(Some(root)) => git::branches(&root).unwrap_or_default(),
                _ => Vec::new(),
            }
        });
        cx.spawn(async move |this, cx| {
            let branches = task.await;
            let _ = this.update(cx, |this, cx| this.finish_branch_load(id, ticket, branches, cx));
        })
        .detach();
    }

    /// The branch list arrived. It is shown only if that same picker is still
    /// open for the session and folder it was asked for; a list that lands after
    /// the picker was closed, reopened for the other tab or moved to another
    /// session is dropped.
    fn finish_branch_load(&mut self, id: i64, ticket: Ticket, branches: Vec<String>, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.iter_mut().find(|s| s.id == id) else { return };
        let Some(asked) = session.git.branches.finish(ticket) else { return };
        let cwd_now = self.active_terminal().read(cx).cwd.clone();
        if branches_wanted(&asked, id, self.picker, self.active_session().id, cwd_now.as_deref()) {
            self.branches = branches;
            cx.notify();
        }
    }

    pub(crate) fn pick_branch(&mut self, picker: Picker, branch: Option<String>, cx: &mut Context<Self>) {
        self.picker = None;
        self.branches.clear();
        let ix = self.active;
        let session = &mut self.sessions[ix];
        match picker {
            Picker::Compare => {
                session.compare = branch;
                session.git.status = None;
                self.diff_view = None;
                self.refresh_git(ix, cx);
            }
            Picker::History => {
                session.history = HistoryState::for_rev(branch);
                self.refresh_history(ix, cx);
            }
        }
        cx.notify();
    }

    /// HEAD (`None`) first, then the repository's branches, each with the branch mark.
    fn picker_options(&self, picker: Picker, cx: &App) -> Vec<Choice<Option<String>>> {
        let head = match picker {
            Picker::Compare => "HEAD (uncommitted changes)",
            Picker::History => "HEAD (current branch)",
        };
        let branch = ui_in(&self.profile, cx).branch;
        std::iter::once(Choice::new(None, head))
            .chain(
                self.branches
                    .iter()
                    .map(|b| Choice::new(Some(b.clone()), b.clone()).mark(IconName::GitBranch, branch)),
            )
            .collect()
    }

    /// Enter in the search box: take the first branch that matches.
    pub(crate) fn pick_first_match(&mut self, cx: &mut Context<Self>) {
        let Some(picker) = self.picker else { return };
        let query = self.picker_search.read(cx).value().to_string();
        if let Some(first) = select::filter(&self.picker_options(picker, cx), &query).into_iter().next() {
            self.pick_branch(picker, first.value, cx);
        }
    }

    /// The picker in a panel card's head, as the data its chip draws from: the rows of a list
    /// are drawn from a callback that can't borrow the workspace, so the chip is described here
    /// and built there (`PickerChip::render`). `prefix` reads muted before the picked branch.
    pub(crate) fn picker_chip(
        &self,
        picker: Picker,
        prefix: &'static str,
        branch: Option<&str>,
        cx: &mut Context<Self>,
    ) -> PickerChip {
        let session = self.active_session();
        let open = self.picker == Some(picker);
        PickerChip {
            picker,
            prefix,
            branch: branch.map(str::to_string),
            current: match picker {
                Picker::Compare => session.compare.clone(),
                Picker::History => session.history.rev.clone(),
            },
            open,
            // The menu is built only while it is open; the head re-renders far more often than that.
            options: if open { self.picker_options(picker, cx) } else { Vec::new() },
            loading: self.branches.is_empty(),
            search: self.picker_search.clone(),
            this: cx.weak_entity(),
        }
    }
}

/// A branch picker's chip and, while it is open, the floating, searchable list of branches
/// under it. It reads `prefix` ("vs", muted) then the picked branch after its mark in
/// `git.branch`, or HEAD.
pub(crate) struct PickerChip {
    picker: Picker,
    prefix: &'static str,
    branch: Option<String>,
    current: Option<String>,
    open: bool,
    options: Vec<Choice<Option<String>>>,
    loading: bool,
    search: Entity<InputState>,
    this: WeakEntity<Workspace>,
}

impl PickerChip {
    pub(crate) fn render(&self, ui: UiColors, cx: &App) -> AnyElement {
        let (picker, prefix) = (self.picker, self.prefix);
        let label = div()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .when(!prefix.is_empty(), |d| d.child(div().flex_none().text_color(ui.muted).child(prefix)))
            .when(self.branch.is_some(), |d| {
                d.child(Icon::new(IconName::GitBranch).xsmall().text_color(ui.branch).flex_none())
            })
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .child(SharedString::from(self.branch.clone().unwrap_or_else(|| "HEAD".to_string()))),
            );
        let id = match picker {
            Picker::Compare => "compare-chip",
            Picker::History => "history-chip",
        };
        let (this, on_change) = (self.this.clone(), self.this.clone());
        let open = self.open;
        let select = SearchSelect::new(
            id,
            ui,
            SelectTrigger::new(SharedString::from(format!("{id}-trigger")), ui, label).compact(),
            &self.search,
            cx,
        )
        .options(self.options.clone())
        .current(self.current.clone())
        .open(open)
        .anchor(Anchor::TopRight)
        .menu_width(px(320.0))
        .compact()
        .on_open_change(move |want_open, window, cx| {
            on_change
                .update(cx, |this, cx| {
                    if (this.picker == Some(picker)) != want_open {
                        this.toggle_picker(picker, window, cx);
                    }
                })
                .ok();
        })
        .on_pick(move |value, _, cx| {
            this.update(cx, |this, cx| this.pick_branch(picker, value.clone(), cx)).ok();
        });
        let select = if self.loading {
            select.hint("Loading branches…")
        } else if picker == Picker::Compare {
            select.hint("Changes since this branch split off from it")
        } else {
            select
        };
        select.into_popover().into_any_element()
    }
}

/// Whether a branch list asked for `asked` by session `id` still belongs on screen:
/// the same picker is open, for the session and the folder in front now.
fn branches_wanted(
    asked: &(Picker, PathBuf),
    id: i64,
    open: Option<Picker>,
    active: i64,
    cwd_now: Option<&Path>,
) -> bool {
    open == Some(asked.0) && id == active && cwd_now == Some(asked.1.as_path())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Picker, branches_wanted};
    use crate::window::features::git::changes::GitState;
    use crate::window::features::git::job::testing::ManualExecutor;

    const CWD: &str = "/work/repo";

    fn asked(picker: Picker) -> (Picker, PathBuf) {
        (picker, PathBuf::from(CWD))
    }

    #[test]
    fn a_list_belongs_to_the_open_picker_of_the_session_and_folder_in_front() {
        let cwd = Some(Path::new(CWD));
        assert!(branches_wanted(&asked(Picker::Compare), 1, Some(Picker::Compare), 1, cwd));
        // Closed meanwhile, or the other tab's picker is open.
        assert!(!branches_wanted(&asked(Picker::Compare), 1, None, 1, cwd));
        assert!(!branches_wanted(&asked(Picker::Compare), 1, Some(Picker::History), 1, cwd));
        // Another session came to the front, or the tab moved to another folder.
        assert!(!branches_wanted(&asked(Picker::Compare), 1, Some(Picker::Compare), 2, cwd));
        assert!(!branches_wanted(
            &asked(Picker::Compare),
            1,
            Some(Picker::Compare),
            1,
            Some(Path::new("/other"))
        ));
        assert!(!branches_wanted(&asked(Picker::Compare), 1, Some(Picker::Compare), 1, None));
    }

    /// Opening a picker asks for its branches; the answer goes through the session's job.
    fn open(
        git: &mut GitState,
        exec: &mut ManualExecutor<(GitState, Vec<String>)>,
        picker: Picker,
        branches: &[&str],
    ) {
        let Some(wanted) = git.branches.begin(asked(picker)) else { return };
        let ticket = wanted.ticket();
        let list: Vec<String> = branches.iter().map(|b| b.to_string()).collect();
        exec.spawn(
            move || list,
            move |(git, shown): &mut (GitState, Vec<String>), list| {
                if git.branches.finish(ticket).is_some() {
                    *shown = list;
                }
            },
        );
    }

    #[test]
    fn a_list_for_a_picker_that_was_replaced_is_dropped() {
        let mut state = (GitState::default(), Vec::new());
        let mut exec = ManualExecutor::default();
        // Compare opened, then History before the first list came back.
        open(&mut state.0, &mut exec, Picker::Compare, &["from-compare"]);
        open(&mut state.0, &mut exec, Picker::History, &["from-history"]);
        // The slower, older answer lands last.
        exec.deliver(1, &mut state);
        exec.deliver(0, &mut state);
        assert_eq!(state.1, ["from-history"]);
        assert_eq!(exec.pending(), 0);
    }

    #[test]
    fn reopening_the_same_picker_waits_for_the_list_already_on_its_way() {
        let mut state = (GitState::default(), Vec::new());
        let mut exec = ManualExecutor::default();
        open(&mut state.0, &mut exec, Picker::Compare, &["main"]);
        open(&mut state.0, &mut exec, Picker::Compare, &["main"]);
        assert_eq!(exec.pending(), 1, "no second fetch");
        exec.deliver(0, &mut state);
        assert_eq!(state.1, ["main"]);
    }
}
