//! Renaming a terminal in its row, or a group on its head ("Rename…" in their menus): the name
//! becomes a field with the current name selected; ↩ or a click elsewhere keeps what was typed, ⎋
//! the old name, and an empty name goes back to what runs in the terminal (a group keeps its old
//! one). A terminal's name is the window's to set (`SidebarEvent::Renamed`); a group's is the list's.
use gpui_kit::component::Sizable;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;

use super::{Sidebar, SidebarEvent};

/// A terminal being renamed: which, and its name field.
pub(super) struct TerminalRename {
    id: i64,
    pub(super) input: Entity<InputState>,
    _events: Subscription,
}

impl Sidebar {
    pub(crate) fn start_terminal_rename(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.input.find(id) else { return };
        let name = self.input.rows[ix].name.to_string();
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, _, cx| match event {
            InputEvent::PressEnter { .. } | InputEvent::Blur => this.finish_terminal_rename(true, cx),
            _ => {}
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.renaming = Some(TerminalRename { id, input, _events: events });
        cx.notify();
    }

    /// Keep the typed name (`keep`) or the old one; the window sets it and gives the keyboard back to
    /// the terminal.
    pub(crate) fn finish_terminal_rename(&mut self, keep: bool, cx: &mut Context<Self>) {
        let Some(renaming) = self.renaming.take() else { return };
        let typed = keep.then(|| renaming.input.read(cx).value().trim().to_string());
        cx.emit(SidebarEvent::Renamed { id: renaming.id, typed });
        cx.notify();
    }

    /// The field that stands in the name's place on terminal `id`'s row, if it is being renamed.
    pub(super) fn rename_field(&self, id: i64, cx: &mut Context<Self>) -> Option<AnyElement> {
        let renaming = self.renaming.as_ref().filter(|renaming| renaming.id == id)?;
        Some(
            div()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.finish_terminal_rename(false, cx);
                    }
                }))
                .child(Input::new(&renaming.input).xsmall())
                .into_any_element(),
        )
    }
}

/// A group being renamed: which, and its name field.
pub(super) struct GroupRename {
    pub(super) group: u32,
    pub(super) input: Entity<InputState>,
    _events: Subscription,
}

impl Sidebar {
    /// Group `group`'s name becomes a field on its head (or on its card in the group switcher).
    pub(crate) fn start_group_rename(&mut self, group: u32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.view.groups.get(group).map(|g| g.name.clone()) else { return };
        // The field made below has the keyboard: nothing here gives it to the terminal meanwhile.
        self.end_group_rename(true, false, window, cx);
        // With the terminals in tabs the groups are only on the switcher's cards.
        if self.input.tabs_on && self.tabs.switcher.is_none() {
            self.start_switcher(window, cx);
        }
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } | InputEvent::Blur => this.finish_group_rename(true, window, cx),
            _ => {}
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.group_renaming = Some(GroupRename { group, input, _events: events });
        cx.notify();
    }

    /// Keep the typed name (`keep`) or the old one, and give the keyboard back to the terminal.
    pub(crate) fn finish_group_rename(&mut self, keep: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.end_group_rename(keep, true, window, cx);
    }

    /// [`Self::finish_group_rename`], with the keyboard handed back to the terminal only when
    /// `refocus`: a rename that is replaced by another keeps the keyboard for it.
    pub(super) fn end_group_rename(
        &mut self,
        keep: bool,
        refocus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(renaming) = self.group_renaming.take() else { return };
        if keep {
            let typed = renaming.input.read(cx).value().to_string();
            self.view.groups.rename(renaming.group, &typed);
            self.changed(cx);
        }
        // A switcher the rename was started in goes with it.
        self.end_switcher(refocus, window, cx);
        if refocus {
            cx.emit(SidebarEvent::FocusTerminal);
        }
        cx.notify();
    }

    /// The field that stands in the name's place on group `group`'s head, if it is being renamed.
    pub(super) fn group_rename_field(&self, group: u32, cx: &mut Context<Self>) -> Option<AnyElement> {
        let renaming = self.group_renaming.as_ref().filter(|renaming| renaming.group == group)?;
        Some(group_name_field(&renaming.input, cx.weak_entity()))
    }
}

/// The name field of a group being renamed: ⎋ drops what was typed.
pub(super) fn group_name_field(input: &Entity<InputState>, this: WeakEntity<Sidebar>) -> AnyElement {
    div()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .capture_key_down(move |event: &KeyDownEvent, window, cx| {
            if event.keystroke.key == "escape" {
                cx.stop_propagation();
                this.update(cx, |sidebar, cx| sidebar.finish_group_rename(false, window, cx)).ok();
            }
        })
        .child(Input::new(input).xsmall())
        .into_any_element()
}
