//! The person's own groups in the list, as the window changes them: folding, putting terminals
//! in and out of groups (a row's menu, a drop on a group's head or beside one of its rows), making
//! and taking apart groups, their colour and pin, and the popover their label opens (name, nine
//! colours, the group's actions). Core keeps the groups and their order (`sidebar::WindowSidebar`,
//! saved with the window); `group_head` draws them and `group_actions` closes and moves them.
use gpui_kit::component::input::Input;
use gpui_kit::component::popover::Popover;
use wormhole_core::sidebar::{GroupColor, Item, Landing};

use super::*;
use crate::select;
use crate::settings::SettingsStore;
use crate::style::PopoverTrigger;
use crate::theme_bridge;

/// A group's popover being open: which group, and its name field.
pub(super) struct GroupEdit {
    pub(super) group: u32,
    pub(super) input: Entity<InputState>,
    _events: Subscription,
}

/// The popover's width: nine 28 pt colours and its padding.
const POPOVER_W: f32 = 276.0;
/// The popover's action rows.
const ACTION_H: f32 = 28.0;

/// The colours a group can wear, as the theme in force draws them.
pub(in crate::workspace) fn group_colors(cx: &App) -> Vec<(GroupColor, Hsla)> {
    let tokens = &cx.global::<SettingsStore>().tokens;
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

    /// A new terminal in group `group` (its + or its menu), where the selected terminal is.
    pub(in crate::workspace) fn new_session_in_group(
        &mut self,
        group: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = self.default_cwd(cx);
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

    /// "Add terminal to new group": a group holding terminal `id` (and its split), its popover open
    /// at once to name it.
    pub(in crate::workspace) fn group_terminal(
        &mut self,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let group = self.group_terminals(&[id], cx);
        self.start_group_edit(group, window, cx);
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

    // ---------------------------------------------------------------- the popover

    /// The group's popover opens on its label, its name selected: what is typed is kept when it
    /// closes (↩, ⎋ or a click elsewhere).
    pub(in crate::workspace) fn start_group_edit(
        &mut self,
        group: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(name) = self.sidebar.view.groups.get(group).map(|g| g.name.clone()) else { return };
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.finish_group_edit(window, cx);
            }
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.sidebar.editing = Some(GroupEdit { group, input, _events: events });
        cx.notify();
    }

    /// Close the popover, keeping the typed name, and give the keyboard back to the terminal.
    pub(in crate::workspace) fn finish_group_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editing) = self.sidebar.editing.take() else { return };
        let name = editing.input.read(cx).value().to_string();
        self.sidebar.view.groups.rename(editing.group, &name);
        self.mark_dirty(cx);
        // A popover opened from a switcher card hangs off the switcher: they end together.
        self.finish_switcher(window, cx);
        window.focus(&self.active_terminal().focus_handle(cx), cx);
    }

    /// What a group's popover is drawn from: copied when the head is drawn, since the popover builds
    /// its content later, without the workspace at hand.
    pub(super) fn popover_snapshot(&self, group: u32, ui: UiColors, cx: &App) -> PopoverSnapshot {
        let edit = self.sidebar.editing.as_ref().filter(|edit| edit.group == group);
        PopoverSnapshot {
            group,
            ui,
            open: edit.is_some(),
            input: edit.map(|edit| edit.input.clone()),
            current: self.sidebar.view.groups.get(group).and_then(|g| g.color()),
            pinned: self.sidebar.view.groups.get(group).is_some_and(|g| g.pinned),
            colors: group_colors(cx),
            bg: select::menu_bg(ui, cx),
        }
    }
}

/// See [`Workspace::popover_snapshot`].
#[derive(Clone)]
pub(super) struct PopoverSnapshot {
    group: u32,
    ui: UiColors,
    open: bool,
    input: Option<Entity<InputState>>,
    current: Option<GroupColor>,
    pinned: bool,
    colors: Vec<(GroupColor, Hsla)>,
    bg: Hsla,
}

/// The label's popover: the name, the nine colours and the group's actions. `trigger` is the label,
/// which the popover hangs off.
pub(super) fn group_popover(
    snap: PopoverSnapshot,
    trigger: PopoverTrigger,
    this: WeakEntity<Workspace>,
) -> AnyElement {
    let PopoverSnapshot { group, ui, open, input, current, pinned, colors, bg } = snap;
    let on_open = this.clone();
    let popover = Popover::new(("group-edit", group as u64))
        .anchor(Anchor::TopLeft)
        .offset(px(6.0))
        .open(open)
        .on_open_change(move |open, window, cx| {
            on_open
                .update(cx, |ws, cx| {
                    if *open {
                        ws.start_group_edit(group, window, cx);
                    } else {
                        ws.finish_group_edit(window, cx);
                    }
                })
                .ok();
        })
        .trigger(trigger);
    select::menu_surface(popover, ui, px(POPOVER_W), bg)
        .p_3()
        .content(move |_, _, _| {
            let action = |id: &'static str, label: &'static str| {
                div()
                    .id((id, group as u64))
                    .role(Role::Button)
                    .aria_label(label)
                    .flex()
                    .items_center()
                    .h(px(ACTION_H))
                    .px_2()
                    .rounded(ui.radius)
                    .cursor_pointer()
                    .text_body()
                    .text_color(ui.text)
                    .hover(move |d| d.bg(ui.hover))
                    .child(label)
            };
            let acting = |run: fn(&mut Workspace, u32, &mut Window, &mut Context<Workspace>)| {
                let this = this.clone();
                move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                    this.update(cx, |ws, cx| {
                        // The popover goes first, keeping the name typed so far.
                        ws.finish_group_edit(window, cx);
                        run(ws, group, window, cx);
                    })
                    .ok();
                }
            };
            div()
                .flex()
                .flex_col()
                .gap_2()
                .children(input.as_ref().map(|input| Input::new(input).small()))
                .child(div().flex().items_center().children(colors.iter().map(|&(color, hsla)| {
                    let this = this.clone();
                    style::color_choice(
                        ("group-color", color as usize),
                        ui,
                        color.label(),
                        Some(hsla),
                        current == Some(color),
                        true,
                    )
                    .on_click(move |_, _, cx| {
                        this.update(cx, |ws, cx| ws.set_group_color(group, Some(color), cx)).ok();
                    })
                })))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            action("group-new-terminal", "New terminal in group")
                                .on_click(acting(|ws, g, w, cx| ws.new_session_in_group(g, w, cx))),
                        )
                        .child(
                            action("group-ungroup", "Ungroup")
                                .on_click(acting(|ws, g, _, cx| ws.ungroup(g, cx))),
                        )
                        .child(
                            action("group-close", "Close group")
                                .on_click(acting(|ws, g, w, cx| ws.close_group(g, w, cx))),
                        )
                        .child(
                            action("group-pin", if pinned { "Unpin group" } else { "Pin group" })
                                .on_click(acting(|ws, g, _, cx| ws.toggle_group_pin(g, cx))),
                        )
                        .child(
                            action("group-move", "Move group to new window")
                                .on_click(acting(|ws, g, w, cx| ws.move_group_to_window(g, w, cx))),
                        ),
                )
        })
        .into_any_element()
}
