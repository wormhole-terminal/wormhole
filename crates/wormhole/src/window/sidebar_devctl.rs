//! devctl's commands on the sidebar: fold a group, make one, what a group's menu does, rename, scroll.
//! The list's own operations are the sidebar's; these are the window-shaped ones a script asks for by
//! row number or group name.

use gpui_kit::*;
use wormhole_core::view::sidebar::GroupColor;

use crate::window::Workspace;

impl Workspace {
    /// Fold the group called `label`, or open it; `false` if there is none.
    pub(crate) fn devctl_fold(&mut self, label: &str, cx: &mut Context<Self>) -> bool {
        let listing = self.session_listing(cx);
        let Some(band) = listing.bands.iter().find(|band| band.label == label) else { return false };
        let group = band.group;
        self.toggle_group_band(group, cx);
        true
    }

    /// A new group named `name` holding the terminals at `rows` (as `state` numbers them, from 0);
    /// its id.
    pub(crate) fn devctl_new_group(&mut self, name: &str, rows: &[usize], cx: &mut Context<Self>) -> u32 {
        let ids: Vec<i64> = rows.iter().filter_map(|&ix| self.sessions.get(ix).map(|s| s.id)).collect();
        let group = self.group_terminals(&ids, cx);
        self.sidebar.update(cx, |sidebar, _| sidebar.view_mut().groups.rename(group, name));
        self.mark_dirty(cx);
        group
    }

    /// The id of the group called `name`.
    fn devctl_group_named(&self, name: &str, cx: &App) -> Result<u32, String> {
        self.sidebar_view(cx)
            .groups
            .iter()
            .find(|group| group.name == name)
            .map(|group| group.id)
            .ok_or_else(|| format!("no group {name:?}"))
    }

    /// devctl `group <name> <rename|pin|ungroup|close|move|color:<colour>|none|folder:<path>|none>`: what
    /// the group's menu does. `rename` makes its name a field.
    pub(crate) fn devctl_group(
        &mut self,
        name: &str,
        action: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let group = self.devctl_group_named(name, cx)?;
        match action {
            "rename" | "edit" => {
                self.sidebar.update(cx, |sidebar, cx| sidebar.start_group_rename(group, window, cx))
            }
            "pin" => self.toggle_group_pin(group, cx),
            "ungroup" => self.with_sidebar(cx, |sidebar, cx| sidebar.ungroup(group, cx)),
            "close" => self.close_group(group, window, cx),
            "move" => self.move_group_to_window(group, window, cx),
            folder if folder.starts_with("folder:") => {
                // The picker can't be driven, so the path comes with the command.
                let folder = folder.trim_start_matches("folder:");
                self.sidebar.update(cx, |sidebar, cx| {
                    sidebar.set_group_folder(group, (folder != "none").then_some(folder), cx)
                });
            }
            color => {
                let color =
                    color.strip_prefix("color:").ok_or_else(|| format!("unknown action {action:?}"))?;
                let found = if color == "none" {
                    None
                } else {
                    Some(GroupColor::from_key(color).ok_or_else(|| format!("no colour {color:?}"))?)
                };
                self.sidebar.update(cx, |sidebar, cx| sidebar.set_group_color(group, found, cx));
            }
        }
        Ok(())
    }

    /// devctl `to-group <row> <name|none>`: put the terminal in the group, or take it out.
    pub(crate) fn devctl_to_group(
        &mut self,
        ix: usize,
        name: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let id = self.sessions.get(ix).map(|session| session.id).ok_or("no such row")?;
        if name == "none" {
            self.with_sidebar(cx, |sidebar, cx| sidebar.remove_from_group(id, cx));
            return Ok(());
        }
        let group = self.devctl_group_named(name, cx)?;
        self.with_sidebar(cx, |sidebar, cx| sidebar.move_to_group(id, group, cx));
        Ok(())
    }

    pub(crate) fn devctl_rename_edit(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.sessions.get(ix).map(|session| session.id) {
            self.start_terminal_rename(id, window, cx);
        }
    }

    pub(crate) fn devctl_rename(&mut self, ix: usize, name: &str, cx: &mut Context<Self>) {
        if let Some(session) = self.sessions.get_mut(ix) {
            session.custom_name = (!name.is_empty()).then(|| name.to_string());
            self.mark_dirty(cx);
        }
    }

    /// Scroll the terminal list by `dy` (negative = up), within its bounds; where it is now and
    /// how far it goes.
    pub(crate) fn devctl_sidebar_scroll(&mut self, dy: f32, cx: &mut Context<Self>) -> String {
        let scroll = self.sidebar.read(cx).scroll().clone();
        let (offset, max) = (scroll.offset(), scroll.max_offset());
        let y = (offset.y - px(dy)).clamp(-max.y, px(0.0));
        scroll.set_offset(point(offset.x, y));
        cx.notify();
        format!("sidebar scroll {} of {}", -f32::from(y), f32::from(max.y))
    }
}
