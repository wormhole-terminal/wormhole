//! Where a new terminal in a group starts, the question that asks for a folder when the group has none,
//! and a group that keeps only the terminals in its folder. The decisions are core's
//! (`view::sidebar::group_folder`); this gathers what they need from the window (each terminal's folder as
//! its summary holds it, made real, and the repository it is in as the session already found it) and acts
//! on the answer: the folder set, terminals outside it taken out of the group, the "no" remembered.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::view::sidebar::display_path;
use wormhole_core::view::sidebar::group_folder::{Fence, Place, keeps, real_folder};
use wormhole_text::{keys, text, whole};

use crate::dialog;
use crate::platform::sheet::{Question, Style};
use crate::style::home_dir;
use crate::window::Workspace;

/// How the question about a group's folder was answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FolderAnswer {
    /// "Use Folder"; `only_in_folder` is the box: the group keeps only the terminals in the folder.
    Use { only_in_folder: bool },
    /// "Not Now" or Esc: the group is not asked again.
    NotNow,
}

impl Workspace {
    /// Where terminal `id` is: its folder made real, the repository it is in, which the session has
    /// already found for the list (it is not looked for again here), and the repository its folder is a
    /// linked worktree of (read from the folder's `.git` file once per refresh, as the list does).
    fn member_place(&self, id: i64) -> Option<Place> {
        let ix = self.find_session(id)?;
        let session = &self.sessions[ix];
        let cwd = session.pane.summary.cwd.as_deref()?;
        let root = session.repo_root.of(cwd);
        Some(Place {
            folder: real_folder(cwd)?,
            repo: session.repo_root.found(cwd).and_then(|root| real_folder(&root)),
            // As the folder's `.git` file names it, not looked up: it can lead anywhere.
            linked_to: self.worktree_of(&root),
        })
    }

    /// The fence of `group` when it keeps only the terminals in its folder.
    fn fence_of(&self, group: u32, cx: &App) -> Option<Fence> {
        self.sidebar_view(cx).groups.fence(group, crate::sidebar::repo_of_folder)
    }

    /// Whether `group` keeps only the terminals in its folder and a terminal started in `cwd` is known
    /// to be outside it (a linked worktree of the folder's repository is inside). A place that can't be
    /// told is not outside.
    pub(super) fn fenced_out(&self, group: u32, cwd: Option<&str>, cx: &App) -> bool {
        let Some(fence) = self.fence_of(group, cx) else { return false };
        let Some((cwd, folder)) = cwd.and_then(|cwd| Some((cwd, real_folder(Path::new(cwd))?))) else {
            return false;
        };
        let root = wormhole_core::git::find_root(Path::new(cwd)).unwrap_or_else(|| PathBuf::from(cwd));
        let linked_to = self.worktree_of(&root);
        !keeps(&fence, [Some(Place { folder, repo: None, linked_to })])
    }

    /// The id of the selected terminal.
    fn selected_id(&self) -> Option<i64> {
        self.sessions.get(self.active).map(|session| session.id)
    }

    /// Where a new terminal in `group` (`None`: in no group) starts: the group's folder if it has
    /// one that is still a directory, else where the group was last used, else where the selected
    /// terminal is. A split's new pane and a terminal in no group keep starting beside the selected one.
    pub(super) fn start_cwd(&self, group: Option<u32>, cx: &App) -> Option<String> {
        let place = group.and_then(|group| {
            let last = self.sidebar.read(cx).last_selected(Some(group));
            let groups = &self.sidebar_view(cx).groups;
            groups.start_place(group, self.selected_id(), last, |id| self.member_place(id))
        });
        place.and_then(|folder| folder.into_os_string().into_string().ok()).or_else(|| self.default_cwd(cx))
    }

    /// The folder to ask about before a terminal opens in `group`, if it is to be asked at all.
    pub(super) fn group_folder_question(&self, group: u32, cx: &App) -> Option<PathBuf> {
        // Without a home folder only `/` is ruled out.
        let home = home_dir().and_then(real_folder).unwrap_or_else(|| PathBuf::from("/"));
        let last = self.sidebar.read(cx).last_selected(Some(group));
        let groups = &self.sidebar_view(cx).groups;
        groups.folder_to_ask_about(group, self.selected_id(), last, |id| self.member_place(id), &home)
    }

    /// A new terminal in group `group` (its + or its menu). A group with no folder that hasn't said
    /// no is asked first, when there is a folder to offer; either way the terminal opens in the
    /// group's folder, else where it was last used.
    pub(super) fn new_session_in_group(&mut self, group: u32, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(folder) = self.group_folder_question(group, cx)
            && self.ask_group_folder(group, folder, window, cx)
        {
            return;
        }
        self.open_in_group(group, window, cx);
    }

    fn open_in_group(&mut self, group: u32, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self.start_cwd(Some(group), cx);
        self.open_session_in(Some(Some(group)), None, cwd, window, cx);
        self.mark_dirty(cx);
    }

    /// Put the question about `folder` for `group` up as a sheet. `false` when this window can't show
    /// one (a test, a window without a native one): nothing is asked or remembered, and the terminal
    /// opens as it would after "Not Now".
    fn ask_group_folder(
        &mut self,
        group: u32,
        folder: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let name = self.sidebar_view(cx).groups.get(group).map(|g| g.name.clone()).unwrap_or_default();
        // The title says which folder by its name; the whole path, which can be long, is the detail's first line.
        let path = display_path(&folder, home_dir());
        let folder_name =
            folder.file_name().map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let title =
            keys::sidebar_folder_ask_title(&dialog::shortened(&folder_name), &dialog::shortened(&name));
        let fence = Fence::new(folder.clone(), crate::sidebar::repo_of_folder);
        let leaving = self.members_leaving(group, &fence, cx).len();
        let detail = match leaving {
            0 => keys::sidebar_folder_ask_detail(&path),
            leaving => keys::sidebar_folder_ask_detail_leaving(whole(leaving), &path),
        };
        let question = Question {
            // A choice about where terminals start, not a warning about losing something.
            style: Style::Informational,
            title: &title,
            detail: &detail,
            confirm: text(keys::SIDEBAR_FOLDER_ASK_USE),
            cancel: text(keys::SIDEBAR_FOLDER_ASK_DECLINE),
            checkbox: text(keys::SIDEBAR_FOLDER_ASK_ONLY),
            checked: false,
        };
        let this = cx.weak_entity();
        dialog::ask_with_box(window, cx, &question, move |answer, window, cx| {
            let answer = match answer {
                Some(only_in_folder) => FolderAnswer::Use { only_in_folder },
                None => FolderAnswer::NotNow,
            };
            this.update(cx, |ws, cx| ws.answer_group_folder(group, &name, &folder, answer, window, cx)).ok();
        })
    }

    /// The question about `folder` for the group `group`, which was called `name` when it was asked, was
    /// answered: "Use Folder" sets it (which pins the group, as Set folder… does) and, with the box,
    /// makes the group keep only the terminals in it; "Not Now" is remembered on the group. Then the
    /// terminal that was asked for opens. A group that went while the sheet was up, or one that has another
    /// by its id since, is not touched: the terminal opens as the + would have without a group.
    pub(super) fn answer_group_folder(
        &mut self,
        group: u32,
        name: &str,
        folder: &Path,
        answer: FolderAnswer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.sidebar_view(cx).groups.get(group).is_none_or(|found| found.name != name) {
            // The person pressed +; with the group gone there is no folder to start in, and the terminal
            // is not put in whichever group the selected one is in.
            let cwd = self.default_cwd(cx);
            self.open_session_in(Some(None), None, cwd, window, cx);
            return;
        }
        match answer {
            FolderAnswer::Use { only_in_folder } => {
                if let Some(folder) = folder.to_str() {
                    self.with_sidebar(cx, |sidebar, cx| sidebar.set_group_folder(group, Some(folder), cx));
                    if only_in_folder {
                        self.set_only_in_folder(group, true, cx);
                    }
                }
            }
            FolderAnswer::NotNow => {
                self.edit_sidebar_view(cx, |view| view.groups.decline_folder(group));
                self.mark_dirty(cx);
            }
        }
        self.open_in_group(group, window, cx);
    }

    /// The terminals of `group` that a group keeping only the terminals in `fence` would let go.
    fn members_leaving(&self, group: u32, fence: &Fence, cx: &App) -> Vec<i64> {
        let split_of = |id: i64| self.find_session(id).and_then(|ix| self.sessions[ix].split);
        let groups = &self.sidebar_view(cx).groups;
        groups.members_outside(group, fence, |id| self.member_place(id), split_of)
    }

    /// Turn on, or off, that `group` keeps only the terminals in its folder. Turned on, the terminals in
    /// other folders leave it now (they stay open, loose where the group stood). Needs a folder.
    pub(super) fn set_only_in_folder(&mut self, group: u32, on: bool, cx: &mut Context<Self>) {
        self.edit_sidebar_view(cx, |view| view.groups.set_only_in_folder(group, on));
        if on {
            self.keep_to_folder(group, cx);
        }
        self.mark_dirty(cx);
    }

    /// Take the terminals out of `group` that it, keeping only the terminals in its folder, lets go: the
    /// ones in another folder (a split goes whole when none of its panes is in it). How many left. Nothing
    /// for a group that doesn't keep only its folder's terminals.
    pub(super) fn keep_to_folder(&mut self, group: u32, cx: &mut Context<Self>) -> usize {
        let Some(fence) = self.fence_of(group, cx) else { return 0 };
        let outside = self.members_leaving(group, &fence, cx);
        if !outside.is_empty() {
            self.with_sidebar(cx, |sidebar, cx| sidebar.remove_all_from_group(&outside, cx));
        }
        outside.len()
    }

    /// A terminal left its split, so the split's other panes may have lost the one that held them inside the
    /// folder of `group` (the group the terminal was in, if that keeps only its folder's terminals): check
    /// the group once the update ends, whichever way the pane left (closed, dragged out, joined to another
    /// split, torn off, moved to another window). Every road out of a split goes through `leave_layout`.
    pub(super) fn note_split_lost(&mut self, id: i64, cx: &mut Context<Self>) {
        let groups = &self.sidebar_view(cx).groups;
        let Some(group) = groups.group_of(id).filter(|&group| groups.fenced_folder(group).is_some()) else {
            return;
        };
        self.sidebar_feed.fence_checks.push(group);
        self.sync_sidebar_soon(cx);
    }

    /// Check the groups [`note_split_lost`](Self::note_split_lost) named.
    pub(super) fn keep_noted_groups(&mut self, cx: &mut Context<Self>) {
        let mut groups = std::mem::take(&mut self.sidebar_feed.fence_checks);
        groups.sort_unstable();
        groups.dedup();
        for group in groups {
            self.keep_to_folder(group, cx);
        }
    }

    /// Terminal `id` is in another folder than it was: it leaves its group when that keeps only the
    /// terminals in its folder. Asked where the window learns of the change (`sync_summary_at`), never
    /// while drawing.
    pub(super) fn folder_changed(&mut self, id: i64, cx: &mut Context<Self>) {
        if let Some(group) = self.sidebar_view(cx).groups.group_of(id) {
            self.keep_to_folder(group, cx);
        }
    }
}
