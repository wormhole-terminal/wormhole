//! What a group's menu does beyond rearranging: close it (asking once if an agent is working in
//! it) and move it, with its splits, to a window of its own.
use std::collections::HashSet;

use wormhole_core::sidebar::{Item, SplitId, WindowSidebar, group_key};
use wormhole_core::store::{SplitRecord, WindowRecord};

use super::super::app::{app_mut, open_workspace_window};
use super::super::dialog;
use super::super::session::{close_group_detail, close_group_title};
use super::*;

impl Workspace {
    /// Close group `group`: every terminal in it ends, and the group goes (a pinned one stays as a
    /// line). One question first when an agent works or waits in it, or a command runs.
    pub(in crate::workspace) fn close_group(
        &mut self,
        group: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(found) = self.sidebar.view.groups.get(group) else { return };
        let (name, members) = (found.name.clone(), found.members.clone());
        let sessions: Vec<&Session> =
            members.iter().filter_map(|&id| self.find_session(id)).map(|ix| &self.sessions[ix]).collect();
        let asking: Vec<&&Session> = sessions.iter().filter(|s| s.needs_close_confirmation(cx)).collect();
        if asking.is_empty() {
            self.close_terminals(&members, window, cx);
            return;
        }
        if self.closing_session_prompt {
            return;
        }
        let running: Vec<(Status, bool)> =
            asking.iter().map(|s| (s.status(cx), s.command_running(cx))).collect();
        let programs: Vec<String> = asking.iter().flat_map(|s| s.running_programs(false, cx)).collect();
        let open_items: usize = sessions.iter().map(|s| s.open_items(cx)).sum();
        let question = dialog::CloseQuestion {
            title: close_group_title(&name).into(),
            detail: close_group_detail(sessions.len(), &running, &programs, open_items).into(),
            confirm: "Close Group",
        };
        self.closing_session_prompt = true;
        cx.notify();
        let (confirm, dismiss) = (cx.weak_entity(), cx.weak_entity());
        dialog::ask_before_closing(
            window,
            cx,
            question,
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
    pub(in crate::workspace) fn move_group_to_window(
        &mut self,
        group: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(found) = self.sidebar.view.groups.get(group).cloned() else { return };
        // The group's terminals and the rest of every split they are in.
        let mut ids: Vec<i64> = Vec::new();
        for &id in &found.members {
            for mate in self.with_split_mates(id) {
                if !ids.contains(&mate) {
                    ids.push(mate);
                }
            }
        }
        let moved: Vec<usize> = ids.iter().filter_map(|&id| self.find_session(id)).collect();
        if moved.is_empty() {
            return;
        }
        let records: Vec<_> = moved.iter().map(|&ix| self.sessions[ix].record(cx)).collect();
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
        if self.sidebar.view.is_group_folded(group) {
            sidebar.folded.insert(group_key(group));
        }
        let layout = Layout::new(self.panes, self.right_tab, sidebar);
        let layout = serde_json::to_string(&layout).ok();

        // Out of this window, the running terminals kept alive for the new one.
        let mut gone: Vec<usize> = moved;
        gone.sort_unstable_by(|a, b| b.cmp(a));
        for ix in gone {
            self.sessions.remove(ix);
        }
        self.splits.retain(|split| !split_ids.contains(&split.id));
        self.sidebar.view.groups.remove(group);
        self.sidebar.view.folded.remove(&group_key(group));
        for &id in &ids {
            self.sidebar.view.groups.leave(id);
        }
        self.active = selected.and_then(|id| self.find_session(id)).unwrap_or(0);
        if self.sessions.is_empty() {
            self.active = usize::MAX;
            self.new_session(window, cx);
        }
        self.list_rearranged(cx);

        let source = window.bounds();
        // Deferred: the new window renders right away, and nothing may read this workspace while
        // it is still being updated.
        cx.defer(move |cx| {
            let state = app_mut(cx);
            let window_id = state.alloc_window_id();
            state.adopted.extend(terminals);
            let record =
                WindowRecord { id: window_id, active, layout, bounds: None, sessions: records, splits };
            open_workspace_window(Some(record), Some(source), cx);
        });
    }
}
