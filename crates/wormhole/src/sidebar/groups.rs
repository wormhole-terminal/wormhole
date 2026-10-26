//! The person's own groups in the list, as the sidebar changes them: folding, putting terminals
//! in and out of groups (a row's menu, a drop on a group's head or beside one of its rows), making
//! and taking apart groups, their colour, pin and the folder their new terminals start in. Core
//! keeps the groups and their order (`sidebar::WindowSidebar`, saved with the window); `group_head`
//! draws them and has their menu and `rename` renames them in place. What also closes or moves
//! terminals (closing a group, moving it to a window of its own) is the window's (`group_ops`).
use std::path::Path;

use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::sidebar::{GroupColor, Item, Landing};

use super::Sidebar;
use crate::settings::store_in;
use crate::style::home_dir;
use crate::theme_bridge;

/// The colours a group can wear, as the theme in force draws them.
pub(crate) fn group_colors(profile: &ProfileId, cx: &App) -> Vec<(GroupColor, Hsla)> {
    let tokens = &store_in(profile, cx).tokens;
    GroupColor::ALL.into_iter().map(|color| (color, theme_bridge::hsla(color.color(tokens)))).collect()
}

impl Sidebar {
    /// Fold an open group, or open a folded one.
    pub(crate) fn toggle_group_band(&mut self, group: u32, cx: &mut Context<Self>) {
        self.view.toggle_group_fold(group);
        self.motion.note_fold(group, self.view.is_group_folded(group));
        self.list_rearranged(cx);
    }

    /// Where a new terminal in `group` starts: the group's folder if it has one that is still a
    /// directory (`None`: the window's choice, where the selected terminal is).
    pub(crate) fn start_folder(&self, group: u32) -> Option<String> {
        self.view.groups.start_folder(group)
    }

    /// Put terminal `id` (with the rest of its split) in group `group`, last in it.
    pub(crate) fn move_to_group(&mut self, id: i64, group: u32, cx: &mut Context<Self>) {
        let ids = self.input.ids();
        for member in self.input.with_split_mates(id) {
            self.view.place_in_group(&ids, member, group, None);
        }
        self.list_rearranged(cx);
    }

    /// Take terminal `id` (with the rest of its split) out of its group: it stands right after it.
    pub(crate) fn remove_from_group(&mut self, id: i64, cx: &mut Context<Self>) {
        let ids = self.input.ids();
        let Some(group) = self.view.groups.group_of(id) else { return };
        let top = self.view.arranged(&ids);
        let after =
            top.iter().position(|&item| item == Item::Group(group)).and_then(|at| top.get(at + 1)).copied();
        for member in self.input.with_split_mates(id) {
            self.view.place_at_top(&ids, Item::Terminal(member), after);
        }
        self.list_rearranged(cx);
    }

    /// A new group holding `terminals` (and the rest of their splits). Its id.
    pub(crate) fn group_terminals(&mut self, terminals: &[i64], cx: &mut Context<Self>) -> u32 {
        let ids = self.input.ids();
        let mut members: Vec<i64> = Vec::new();
        for &id in terminals {
            for member in self.input.with_split_mates(id) {
                if !members.contains(&member) {
                    members.push(member);
                }
            }
        }
        let group = self.view.new_group(&ids, &members);
        self.list_rearranged(cx);
        group
    }

    /// "Add terminal to new group": a group holding terminal `id` (and its split), its name a field
    /// at once to name it.
    pub(super) fn group_terminal(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        let group = self.group_terminals(&[id], cx);
        self.start_group_rename(group, window, cx);
    }

    /// Take group `group` apart; its terminals stand where it stood.
    pub(crate) fn ungroup(&mut self, group: u32, cx: &mut Context<Self>) {
        let ids = self.input.ids();
        self.view.ungroup(&ids, group);
        self.list_rearranged(cx);
    }

    pub(crate) fn set_group_color(&mut self, group: u32, color: Option<GroupColor>, cx: &mut Context<Self>) {
        self.view.groups.set_color(group, color);
        self.changed(cx);
    }

    /// Give group `group` a colour of its own, as the system's colour panel chose it.
    pub(crate) fn set_group_custom_color(&mut self, group: u32, color: Hsla, cx: &mut Context<Self>) {
        let rgba = Rgba::from(color);
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        self.view.groups.set_custom_color(group, [byte(rgba.r), byte(rgba.g), byte(rgba.b)]);
        self.changed(cx);
    }

    /// The group's folder as the list shows it (`~/Projects/app`), if it has one.
    pub(crate) fn group_folder_label(&self, group: u32) -> Option<String> {
        let folder = self.view.groups.get(group)?.folder.as_deref()?;
        Some(wormhole_core::view::sidebar::display_path(Path::new(folder), home_dir()))
    }

    /// The group's folder as its head's second line reads: `group_folder_label` cut from the front to
    /// fit, as a row's place is.
    pub(super) fn group_folder_line(&self, group: u32) -> Option<String> {
        let folder = self.view.groups.get(group)?.folder.as_deref()?;
        wormhole_core::view::sidebar::wording::place_detail(
            Some(Path::new(folder)),
            home_dir(),
            super::rows::PLACE_CHARS,
        )
    }

    /// "Set folder…": ask for a folder, and new terminals in group `group` start there.
    pub(super) fn choose_group_folder(&mut self, group: u32, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return };
            let Some(folder) = paths.into_iter().next() else { return };
            this.update(cx, |sidebar, cx| {
                sidebar.set_group_folder(group, Some(&folder.to_string_lossy()), cx)
            })
            .ok();
        })
        .detach();
    }

    /// Set the folder new terminals in group `group` start in, or take it away (`None`).
    pub(crate) fn set_group_folder(&mut self, group: u32, folder: Option<&str>, cx: &mut Context<Self>) {
        self.view.groups.set_folder(group, folder);
        self.changed(cx);
    }

    /// Pin group `group` (it stays, as a line, when its last terminal closes) or let it go again.
    pub(crate) fn toggle_group_pin(&mut self, group: u32, cx: &mut Context<Self>) {
        let pinned = self.view.groups.get(group).is_some_and(|group| group.pinned);
        self.view.groups.set_pinned(group, !pinned);
        self.changed(cx);
    }

    /// Put what a drop landed where it landed: a terminal (with its split, if `with_mates`) or a
    /// group.
    pub(crate) fn land_terminal(
        &mut self,
        id: i64,
        landing: Landing,
        with_mates: bool,
        cx: &mut Context<Self>,
    ) {
        let ids = self.input.ids();
        let members = if with_mates { self.input.with_split_mates(id) } else { vec![id] };
        for member in members {
            self.view.land(&ids, member, landing);
        }
        self.list_rearranged(cx);
    }

    pub(super) fn land_group(&mut self, group: u32, landing: Landing, cx: &mut Context<Self>) {
        let ids = self.input.ids();
        self.view.land_group(&ids, group, landing);
        self.list_rearranged(cx);
    }

    /// Flash terminal `id`'s row once: it just moved to another group.
    pub(crate) fn flash(&mut self, id: i64) {
        self.flash = Some((id, self.flash.map_or(0, |(_, n)| n + 1)));
    }
}
