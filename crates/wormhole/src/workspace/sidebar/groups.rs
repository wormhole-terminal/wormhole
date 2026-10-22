//! The person's own groups in the list, as the window changes them: folding, putting terminals
//! in and out of groups (a row's menu, a drop on a group's head or beside one of its rows), making
//! and taking apart groups, their colour, pin and the folder their new terminals start in. Core
//! keeps the groups and their order (`sidebar::WindowSidebar`, saved with the window); `group_head`
//! draws them and has their menu, `rename` renames them in place and `group_actions` closes and
//! moves them.
use wormhole_core::sidebar::{GroupColor, Item, Landing};

use super::*;
use crate::theme_bridge;

/// The colours a group can wear, as the theme in force draws them.
pub(in crate::workspace) fn group_colors(profile: &ProfileId, cx: &App) -> Vec<(GroupColor, Hsla)> {
    let tokens = &store_in(profile, cx).tokens;
    GroupColor::ALL.into_iter().map(|color| (color, theme_bridge::hsla(color.color(tokens)))).collect()
}

impl Workspace {
    /// Fold an open group, or open a folded one.
    pub(in crate::workspace) fn toggle_group_band(&mut self, group: u32, cx: &mut Context<Self>) {
        self.sidebar.view.toggle_group_fold(group);
        self.list_rearranged(cx);
    }

    /// The group the selected terminal is in: where a new terminal goes.
    pub(in crate::workspace) fn active_group(&self) -> Option<u32> {
        self.sessions.get(self.active).and_then(|session| self.sidebar.view.groups.group_of(session.id))
    }

    /// Terminal `id` and the rest of its split: they move, join and leave groups together.
    pub(super) fn with_split_mates(&self, id: i64) -> Vec<i64> {
        let Some(ix) = self.find_session(id) else { return Vec::new() };
        match self.sessions[ix].split {
            Some(split) => self.members(split).into_iter().map(|member| self.sessions[member].id).collect(),
            None => vec![id],
        }
    }

    /// Where a new terminal in `group` (`None`: in no group) starts: the group's folder if it has
    /// one that is still a directory, else where the selected terminal is. A split's new pane and
    /// a terminal in no group never ask, so they keep starting beside the selected one.
    pub(in crate::workspace) fn start_cwd(&self, group: Option<u32>, cx: &App) -> Option<String> {
        group.and_then(|group| self.sidebar.view.groups.start_folder(group)).or_else(|| self.default_cwd(cx))
    }

    /// A new terminal in group `group` (its + or its menu), in the group's folder if it has one,
    /// else where the selected terminal is.
    pub(in crate::workspace) fn new_session_in_group(
        &mut self,
        group: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = self.start_cwd(Some(group), cx);
        self.open_session_in(Some(Some(group)), None, cwd, window, cx);
        self.mark_dirty(cx);
    }

    /// Put terminal `id` (with the rest of its split) in group `group`, last in it.
    pub(in crate::workspace) fn move_to_group(&mut self, id: i64, group: u32, cx: &mut Context<Self>) {
        let ids = self.session_ids();
        for member in self.with_split_mates(id) {
            self.sidebar.view.place_in_group(&ids, member, group, None);
        }
        self.list_rearranged(cx);
    }

    /// Take terminal `id` (with the rest of its split) out of its group: it stands right after it.
    pub(in crate::workspace) fn remove_from_group(&mut self, id: i64, cx: &mut Context<Self>) {
        let ids = self.session_ids();
        let Some(group) = self.sidebar.view.groups.group_of(id) else { return };
        let top = self.sidebar.view.arranged(&ids);
        let after =
            top.iter().position(|&item| item == Item::Group(group)).and_then(|at| top.get(at + 1)).copied();
        for member in self.with_split_mates(id) {
            self.sidebar.view.place_at_top(&ids, Item::Terminal(member), after);
        }
        self.list_rearranged(cx);
    }

    /// A new group holding `terminals` (and the rest of their splits). Its id.
    pub(in crate::workspace) fn group_terminals(&mut self, terminals: &[i64], cx: &mut Context<Self>) -> u32 {
        let ids = self.session_ids();
        let mut members: Vec<i64> = Vec::new();
        for &id in terminals {
            for member in self.with_split_mates(id) {
                if !members.contains(&member) {
                    members.push(member);
                }
            }
        }
        let group = self.sidebar.view.new_group(&ids, &members);
        self.list_rearranged(cx);
        group
    }

    /// "Add terminal to new group": a group holding terminal `id` (and its split), its name a field
    /// at once to name it.
    pub(in crate::workspace) fn group_terminal(
        &mut self,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let group = self.group_terminals(&[id], cx);
        self.start_group_rename(group, window, cx);
    }

    /// Take group `group` apart; its terminals stand where it stood.
    pub(in crate::workspace) fn ungroup(&mut self, group: u32, cx: &mut Context<Self>) {
        let ids = self.session_ids();
        self.sidebar.view.ungroup(&ids, group);
        self.list_rearranged(cx);
    }

    pub(in crate::workspace) fn set_group_color(
        &mut self,
        group: u32,
        color: Option<GroupColor>,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.view.groups.set_color(group, color);
        self.mark_dirty(cx);
    }

    /// Give group `group` a colour of its own, as the system's colour panel chose it.
    pub(in crate::workspace) fn set_group_custom_color(
        &mut self,
        group: u32,
        color: Hsla,
        cx: &mut Context<Self>,
    ) {
        let rgba = Rgba::from(color);
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        self.sidebar.view.groups.set_custom_color(group, [byte(rgba.r), byte(rgba.g), byte(rgba.b)]);
        self.mark_dirty(cx);
    }

    /// The group's folder as the list shows it (`~/Projects/app`), if it has one.
    pub(in crate::workspace) fn group_folder_label(&self, group: u32) -> Option<String> {
        let folder = self.sidebar.view.groups.get(group)?.folder.as_deref()?;
        Some(wormhole_core::sidebar::display_path(Path::new(folder), home_dir()))
    }

    /// The group's folder as its head's second line reads: `group_folder_label` cut from the front to
    /// fit, as a row's place is.
    pub(in crate::workspace) fn group_folder_line(&self, group: u32) -> Option<String> {
        let folder = self.sidebar.view.groups.get(group)?.folder.as_deref()?;
        wormhole_core::sidebar::text::place_detail(
            Some(Path::new(folder)),
            home_dir(),
            super::rows::PLACE_CHARS,
        )
    }

    /// "Set folder…": ask for a folder, and new terminals in group `group` start there.
    pub(in crate::workspace) fn choose_group_folder(&mut self, group: u32, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return };
            let Some(folder) = paths.into_iter().next() else { return };
            this.update(cx, |ws, cx| ws.set_group_folder(group, Some(&folder.to_string_lossy()), cx)).ok();
        })
        .detach();
    }

    /// Set the folder new terminals in group `group` start in, or take it away (`None`).
    pub(in crate::workspace) fn set_group_folder(
        &mut self,
        group: u32,
        folder: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.view.groups.set_folder(group, folder);
        self.mark_dirty(cx);
    }

    /// Pin group `group` (it stays, as a line, when its last terminal closes) or let it go again.
    pub(in crate::workspace) fn toggle_group_pin(&mut self, group: u32, cx: &mut Context<Self>) {
        let pinned = self.sidebar.view.groups.get(group).is_some_and(|group| group.pinned);
        self.sidebar.view.groups.set_pinned(group, !pinned);
        self.mark_dirty(cx);
    }

    /// Put what a drop landed where it landed: a terminal (with its split, if `with_mates`) or a
    /// group.
    pub(in crate::workspace) fn land_terminal(
        &mut self,
        id: i64,
        landing: Landing,
        with_mates: bool,
        cx: &mut Context<Self>,
    ) {
        let ids = self.session_ids();
        let members = if with_mates { self.with_split_mates(id) } else { vec![id] };
        for member in members {
            self.sidebar.view.land(&ids, member, landing);
        }
        self.list_rearranged(cx);
    }

    pub(in crate::workspace) fn land_group(&mut self, group: u32, landing: Landing, cx: &mut Context<Self>) {
        let ids = self.session_ids();
        self.sidebar.view.land_group(&ids, group, landing);
        self.list_rearranged(cx);
    }
}
