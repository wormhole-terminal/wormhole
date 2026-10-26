//! What a group's menu does to terminals and windows beyond rearranging the list (`sidebar::groups`
//! does that): close it (asking once if an agent is working in it) and move it, with its splits, to a
//! window of its own.
use std::collections::HashSet;

use gpui_kit::*;
use wormhole_core::ipc::control::event::{Body, Event, Topic};
use wormhole_core::storage::store::{SplitRecord, WindowRecord};
use wormhole_core::view::sidebar::{Item, SplitId, WindowSidebar, group_key};

use crate::app_state::app_mut;
use crate::dialog;
use crate::terminal_view::Status;
use crate::window::feature::{CloseAlso, CloseOf};
use crate::window::feature_host::extras_to_ask;
use crate::window::session::Session;
use crate::window::session::{close_group_detail, close_group_title};
use crate::window::window_lifecycle::open_workspace_window_in;
use crate::window::{Layout, Workspace};

impl Workspace {
    /// Close group `group`: every terminal in it ends, and the group goes (a pinned one stays as a
    /// line). One question first when an agent works or waits in it, or a command runs.
    pub(super) fn close_group(&mut self, group: u32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(found) = self.sidebar_view(cx).groups.get(group) else { return };
        let (name, members) = (found.name.clone(), found.members.clone());
        let sessions: Vec<&Session> =
            members.iter().filter_map(|&id| self.find_session(id)).map(|ix| &self.sessions[ix]).collect();
        let asking: Vec<&&Session> = sessions.iter().filter(|s| s.needs_close_confirmation(cx)).collect();
        // A feature may have more to ask (closing a terminal in a worktree Wormhole made asks about the
        // worktree; so does closing the group it is in).
        let extras = self.close_extras(&members, CloseOf::Group, cx);
        if asking.is_empty() && extras.is_empty() {
            self.close_terminals(&members, window, cx);
            return;
        }
        if self.closing_session_prompt {
            return;
        }
        let running: Vec<(Status, bool)> =
            asking.iter().map(|s| (s.status(cx), s.command_running(cx))).collect();
        let programs: Vec<String> = asking.iter().flat_map(|s| s.running_programs(false, cx)).collect();
        let stakes = self.close_stakes(&members, cx);
        let mut detail = close_group_detail(sessions.len(), &running, &programs, &stakes);
        let (note, also) = extras_to_ask(extras);
        if !note.is_empty() {
            detail.push(' ');
            detail.push_str(&note);
        }
        let question = dialog::CloseQuestion {
            title: close_group_title(&name).into(),
            detail: detail.into(),
            confirm: "Close Group",
        };
        self.closing_session_prompt = true;
        cx.notify();
        let (confirm, remove, dismiss) = (cx.weak_entity(), cx.weak_entity(), cx.weak_entity());
        let close_members = members.clone();
        let also: Option<dialog::Also> = also.map(|CloseAlso { label, then }| {
            let on_also: dialog::OnConfirm = std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let _ = remove.update(cx, |this, cx| this.close_terminals(&close_members, window, cx));
                then(cx);
            });
            (label, on_also)
        });
        dialog::ask_before_closing_with(
            window,
            cx,
            question,
            also,
            move |window, cx| {
                let _ = confirm.update(cx, |this, cx| this.close_terminals(&members, window, cx));
            },
            move |cx| {
                let _ = dismiss.update(cx, |this, cx| {
                    this.closing_session_prompt = false;
                    cx.notify();
                });
            },
        );
    }

    /// Close the terminals with these ids, whichever are still here.
    fn close_terminals(&mut self, ids: &[i64], window: &mut Window, cx: &mut Context<Self>) {
        for &id in ids {
            if let Some(ix) = self.find_session(id) {
                self.close_session(ix, window, cx);
            }
        }
    }

    /// Move group `group` to a window of its own, its terminals running on (the new window takes
    /// the live terminals) and the splits they are in along with them. An empty pinned group has
    /// nothing to move.
    pub(crate) fn move_group_to_window(&mut self, group: u32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(found) = self.sidebar_view(cx).groups.get(group).cloned() else { return };
        // The group's terminals and the rest of every split they are in.
        let mut ids: Vec<i64> = Vec::new();
        let input = self.sidebar_input(cx);
        for &id in &found.members {
            for mate in input.with_split_mates(id) {
                if !ids.contains(&mate) {
                    ids.push(mate);
                }
            }
        }
        let moved: Vec<usize> = ids.iter().filter_map(|&id| self.find_session(id)).collect();
        if moved.is_empty() {
            return;
        }
        let records: Vec<_> = moved.iter().map(|&ix| self.session_record(ix, cx)).collect();
        let terminals: Vec<_> =
            moved.iter().map(|&ix| (self.sessions[ix].id, self.sessions[ix].terminal().clone())).collect();
        let split_ids: HashSet<SplitId> = moved.iter().filter_map(|&ix| self.sessions[ix].split).collect();
        let splits: Vec<SplitRecord> =
            self.split_records().into_iter().filter(|record| split_ids.contains(&record.id)).collect();
        let selected = self.sessions.get(self.active).map(|session| session.id);
        let active = selected.and_then(|id| ids.iter().position(|&moved| moved == id)).unwrap_or(0);

        // The new window's sidebar: the group, folded as it was, standing alone.
        let mut sidebar = WindowSidebar::default();
        sidebar.groups.0.push(found.clone());
        sidebar.items = vec![Item::Group(group)];
        if self.sidebar_view(cx).is_group_folded(group) {
            sidebar.folded.insert(group_key(group));
        }
        let layout = Layout::new(self.panes, self.right_tab, sidebar);
        let layout = serde_json::to_string(&layout).ok();

        // Out of this window, the running terminals kept alive for the new one.
        let mut gone: Vec<usize> = moved;
        gone.sort_unstable_by(|a, b| b.cmp(a));
        for ix in gone {
            let left = self.sessions.remove(ix);
            self.features_left(left.id, cx);
        }
        self.splits.retain(|split| !split_ids.contains(&split.id));
        self.edit_sidebar_view(cx, |view| {
            view.groups.remove(group);
            view.folded.remove(&group_key(group));
            for &id in &ids {
                view.groups.leave(id);
            }
        });
        self.active = selected.and_then(|id| self.find_session(id)).unwrap_or(0);
        if self.sessions.is_empty() {
            self.active = usize::MAX;
            self.new_session(window, cx);
        }
        self.with_sidebar(cx, |sidebar, cx| sidebar.list_rearranged(cx));

        let source = window.bounds();
        // The new window stays in this window's profile: its database, its environment.
        let profile = self.profile.clone();
        let (from, moved_ids) = (self.window_id, ids.clone());
        // Deferred: the new window renders right away, and nothing may read this workspace while
        // it is still being updated.
        cx.defer(move |cx| {
            let state = app_mut(cx);
            let window_id = state.alloc_window_id();
            state.adopted.extend(terminals);
            let record =
                WindowRecord { id: window_id, active, layout, bounds: None, sessions: records, splits };
            open_workspace_window_in(&profile, Some(record), Some(source), cx);
            // They are the same terminals, running on: they moved, they did not open.
            for terminal in moved_ids {
                crate::automation::publish(cx, Topic::Terminals, || Event {
                    window: Some(window_id),
                    terminal: Some(terminal),
                    body: Body::TerminalMoved { from },
                });
            }
        });
    }
}
