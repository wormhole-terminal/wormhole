//! Split panes on screen: the selected terminal's split drawn as its terminals side by
//! side or stacked, each with a header once split and the dividers over their shared edges. The
//! tree, its geometry and the presets are core's (`wormhole_core::view::panes`); the model of who is in
//! which split is `splits.rs`. This draws the tree, runs the zoom, the divider drag, the pane menu
//! and the keys between panes, and keeps the selection in step with the keyboard: the keyboard
//! coming into a pane selects its row. A terminal that is in no split draws as it always did
//! (`layout.rs`): no headers, the terminal under the floating title bar.

pub(in crate::window) mod drop;
pub(in crate::window) mod opening;
mod render;
pub(in crate::window) mod splits;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::storage::store::SessionRecord;
use wormhole_core::view::panes::{Axis, Direction, DividerId, PANE_MIN_W, PaneId, Preset, TabPanes};
use wormhole_core::view::sidebar::SplitId;

use crate::app_state::app;
use crate::app_state::attention::{self, Attention};
use crate::menubar::model::KeyEquivalent;
use crate::platform::{self, Item, Menu};
use crate::style;
use crate::terminal_view::TerminalEvent;
use crate::window::Workspace;

/// A drag on a divider, from the press to the release.
pub(super) struct PaneDrag {
    /// The split whose divider it is (the selected one; it stays so while the button is down).
    split: SplitId,
    divider: DividerId,
    axis: Axis,
    /// The panes as they were when the button went down: Esc goes back to them.
    before: TabPanes,
}

impl PaneDrag {
    /// The pointer while the drag is on: sideways for a vertical divider, up and down for a
    /// horizontal one.
    pub(super) fn cursor(&self) -> CursorStyle {
        match self.axis {
            Axis::Row => CursorStyle::ResizeLeftRight,
            Axis::Column => CursorStyle::ResizeUpDown,
        }
    }
}

impl Workspace {
    /// The keyboard came into the terminal with id `id` (a click, ⌥⌘arrow, F6, a split): its row
    /// is the selected one now, with its split shown. A finished agent there turns idle and its
    /// notification goes.
    pub(super) fn pane_focused(&mut self, id: PaneId, _: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.find_session(id) else { return };
        if ix != self.active {
            self.select(ix, cx);
            self.dirty = true;
        }
        self.sessions[ix].terminal().update(cx, |t, _| t.acknowledge());
        self.sync_summary_at(ix, cx);
        attention::tell(Attention::Seen(id), cx);
        cx.notify();
    }

    /// What a terminal asks of the window: its exited shell's bar has a Restart and a Close.
    pub(crate) fn on_terminal_event(
        &mut self,
        id: i64,
        event: TerminalEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.find_session(id) else { return };
        match event {
            // Taken when the terminal notified (`Pane::new`), which is also when it said so.
            TerminalEvent::SummaryChanged => {}
            // Its row names the host it has now (none, after a fallback or a lost host).
            TerminalEvent::HostChanged => self.dirty = true,
            // Not left for the next minute's save: what was cleared may have been a secret. Only
            // the copy on disk: a hosted terminal's session host is told over the wire
            // (`ClearHistory`, protocol minor 2; a host of minor 1 skips it and keeps its history).
            TerminalEvent::Cleared => {
                if let Some(store) = app(cx).store_of(&self.profile)
                    && let Err(err) = store.clear_scrollback(id)
                {
                    tracing::warn!("clearing terminal {id}'s saved text failed: {err:#}");
                }
            }
            // The workspace draws the find bar.
            TerminalEvent::FindChanged => {
                self.sync_summary_at(ix, cx);
                cx.notify();
            }
            TerminalEvent::Restart => self.restart_pane(ix, window, cx),
            // Found out by a frame (the grid is laid out in it), and the bar was built before it.
            TerminalEvent::Resized => {
                self.grid_resized(ix, cx);
                // Its cell may be another (a font or its size changed): the window's steps are the cell's.
                if ix == self.active {
                    self.sync_resize_increments(window, cx);
                }
            }
            // A program (with `allow_window_ops`) asks the window for something: it is read as
            // being about the pane that asked.
            TerminalEvent::WindowOp(op) => {
                let terminal = self.sessions[ix].terminal().clone();
                let pane = terminal.read(cx).pane_size();
                let done = crate::window_ops::apply(op, pane, window, cx);
                terminal.update(cx, |terminal, cx| terminal.window_op_done(done, cx));
            }
            // Told only while someone watches for them (`wh events`).
            TerminalEvent::Bell => self.publish_bell(id, cx),
            TerminalEvent::Command(mark) => self.publish_command(id, mark, cx),
            TerminalEvent::Printed => self.publish_printed(ix, cx),
            TerminalEvent::Activate => self.focus_pane(id, window, cx),
            // The bar is only on screen in the shown terminals.
            TerminalEvent::Close => {
                if self.on_screen(ix) {
                    self.request_close_session(ix, window, cx);
                }
            }
        }
    }

    /// A new shell in a terminal whose shell has ended, in the folder it was last in and with the
    /// profile it was opened with. The terminal keeps its id, so an agent's hooks find it again.
    pub(super) fn restart_pane(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let old = &self.sessions[ix];
        let cwd = old.terminal().read(cx).cwd.as_ref().map(|p| p.to_string_lossy().into_owned());
        let mut record = SessionRecord::new(old.id, None, cwd);
        record.shell = old.pane.shell.clone();
        let (pane, new_host) = self.make_pane(&record, window, cx);
        let focus = pane.terminal.focus_handle(cx);
        self.sessions[ix].pane = pane;
        if ix == self.active {
            window.focus(&focus, cx);
        }
        // A new host, with a new id: the row names it now.
        if let Some(host) = new_host {
            self.save_host_row(ix, &host, cx);
        }
        self.sync_host_visibility(cx);
        self.dirty = true;
        cx.notify();
    }

    /// ⌘W: close the selected terminal, also inside a split (its pane goes). It asks first, with
    /// the platform's own sheet, while something is still running in it.
    pub(crate) fn close_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.request_close_session(self.active, window, cx);
    }

    /// Close the terminal with id `id` (a pane header's ×, Close pane, its exited shell's Close):
    /// its shell ends, asking first by the same rules as the sidebar's ×.
    pub(crate) fn close_pane(&mut self, id: PaneId, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.find_session(id) {
            self.request_close_session(ix, window, cx);
        }
    }

    /// Select the terminal with id `id` and give it the keyboard (a click on its header, ⌥⌘arrow,
    /// the control socket): its split comes on screen with it.
    pub(crate) fn focus_pane(&mut self, id: PaneId, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.find_session(id) {
            self.activate(ix, window, cx);
        }
    }

    /// ⌥⌘← → ↑ ↓: the keyboard to the pane beside the focused one; at the split's edge nothing
    /// happens.
    pub(crate) fn focus_pane_toward(
        &mut self,
        direction: Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (w, h) = self.pane_area_size();
        let Some(next) = self.active_split().and_then(|split| split.layout.neighbour(direction, w, h)) else {
            return;
        };
        self.focus_pane(next, window, cx);
    }

    /// ⇧⌘↩: the focused pane fills the area, or the panes come back.
    pub(crate) fn toggle_zoom(&mut self, cx: &mut Context<Self>) {
        if self.active_split_mut().is_some_and(|split| split.layout.toggle_zoom()) {
            self.mark_dirty(cx);
        }
    }

    /// The menu bar and the palette: ⌘W closes a pane in a split, Zoom Pane's title, the Layout
    /// check mark, and what the window has no room or no panes for.
    pub(super) fn pane_menu_state(&self, state: &mut keymap::menus::MenuState) {
        use ActionId::*;
        let split = self.active_split();
        state.split = split.is_some();
        if split.is_none_or(|split| split.layout.zoomed().is_none()) {
            state.off.push(ZoomPane);
        }
        if split.is_none() {
            state.disabled.extend([
                ZoomPane,
                LeaveSplit,
                FocusPaneLeft,
                FocusPaneRight,
                FocusPaneUp,
                FocusPaneDown,
            ]);
        }
        if !self.can_split(Axis::Row) {
            state.disabled.push(SplitRight);
        }
        if !self.can_split(Axis::Column) {
            state.disabled.push(SplitDown);
        }
        // A terminal on its own is the single-pane layout.
        let shape = split.map_or(Some(Preset::Single), |split| split.layout.preset());
        for preset in Preset::ALL {
            let id = layout_action(preset);
            if shape == Some(preset) {
                state.checked.push(id);
            }
            if !self.can_layout(preset) {
                state.disabled.push(id);
            }
        }
    }

    // ---------------------------------------------------------------- dividers

    fn start_divider_drag(
        &mut self,
        split: SplitId,
        divider: DividerId,
        axis: Axis,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let Some(layout) = self.split_mut(split).map(|s| &mut s.layout) else { return };
        if event.click_count >= 2 {
            layout.equalize(&divider);
            self.pane_drag = None;
            self.mark_dirty(cx);
            return;
        }
        let before = layout.clone();
        self.pane_drag = Some(PaneDrag { split, divider, axis, before });
        cx.notify();
    }

    pub(super) fn on_divider_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = &self.pane_drag else { return };
        if event.pressed_button != Some(MouseButton::Left) {
            // Released outside the window.
            self.pane_drag = None;
            self.mark_dirty(cx);
            return;
        }
        let area = self.pane_area;
        let (w, h) = self.pane_area_size();
        let position = match drag.axis {
            Axis::Row => f32::from(event.position.x - area.origin.x),
            Axis::Column => f32::from(event.position.y - area.origin.y),
        };
        let (split, divider, min_h) = (drag.split, drag.divider.clone(), self.pane_min_h());
        if let Some(split) = self.split_mut(split) {
            split.layout.drag(&divider, position, w, h, PANE_MIN_W, min_h);
        }
        cx.notify();
    }

    pub(super) fn on_divider_end(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.pane_drag.take().is_some() {
            self.mark_dirty(cx);
        }
    }

    /// Esc while dragging a divider: the panes go back to where the drag began. Returns whether a
    /// drag was going on.
    pub(super) fn cancel_pane_drag(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(drag) = self.pane_drag.take() else { return false };
        if let Some(split) = self.split_mut(drag.split) {
            split.layout = drag.before;
        }
        cx.notify();
        true
    }

    /// The pointer entered or left a divider: its line lights up after a short hover, as a
    /// panel's edge does, so crossing it on the way somewhere else doesn't flash it.
    fn hover_divider(&mut self, divider: DividerId, hovered: bool, cx: &mut Context<Self>) {
        if !hovered {
            self.divider_hover = None;
            if self.divider_hot.take().is_some() {
                cx.notify();
            }
            return;
        }
        self.divider_hover = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(style::motion::RESIZE_HANDLE_HOVER_DELAY).await;
            this.update(cx, |this, cx| {
                this.divider_hot = Some(divider);
                cx.notify();
            })
            .ok();
        }));
    }

    // ---------------------------------------------------------------- drawing

    /// Measures the area the panes share (the terminal's surface): what
    /// a divider's drag, ⌥⌘arrows and the room for a split are worked out in.
    pub(super) fn pane_probe(&self, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity().downgrade();
        canvas(
            move |bounds, _, cx| {
                this.update(cx, |this, _| this.pane_area = bounds).ok();
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .into_any_element()
    }

    /// Pane `id`'s menu: Split right, Split down, Zoom pane, Move out of split, then Close pane,
    /// each with its key. Every entry acts on this pane, which it gives the keyboard first.
    fn pane_menu(&self, id: PaneId, cx: &mut Context<Self>) -> Menu {
        let zoomed = self.active_split().is_some_and(|split| split.layout.zoomed() == Some(id));
        let split = self.active_split().is_some();
        let (right, down) = (self.can_split(Axis::Row), self.can_split(Axis::Column));
        pane_menu(id, PaneFacts { zoomed, split, right, down }, &cx.weak_entity())
    }
}

/// What a pane's menu depends on.
struct PaneFacts {
    zoomed: bool,
    /// The pane is in a split.
    split: bool,
    /// The pane can be split to its right / below.
    right: bool,
    down: bool,
}

/// The menu of pane `id`; every entry acts on `this`.
fn pane_menu(id: PaneId, facts: PaneFacts, this: &WeakEntity<Workspace>) -> Menu {
    let PaneFacts { zoomed, split, right, down } = facts;
    let item = |label: &'static str,
                action: ActionId,
                enabled: bool,
                run: fn(&mut Workspace, PaneId, &mut Window, &mut Context<Workspace>)| {
        Item::new(label)
            .key(keymap::key(action).and_then(KeyEquivalent::from_spec))
            .enabled(enabled)
            .on_select(platform::run_on(this, move |ws, window, cx| {
                ws.focus_pane(id, window, cx);
                run(ws, id, window, cx);
            }))
    };
    Menu::new()
        .item(item("Split right", ActionId::SplitRight, right, |ws, _, window, cx| {
            ws.split_pane(Axis::Row, window, cx)
        }))
        .item(item("Split down", ActionId::SplitDown, down, |ws, _, window, cx| {
            ws.split_pane(Axis::Column, window, cx)
        }))
        .item(item(
            if zoomed { "Unzoom pane" } else { "Zoom pane" },
            ActionId::ZoomPane,
            split,
            |ws, _, _, cx| ws.toggle_zoom(cx),
        ))
        .item(item("Move out of split", ActionId::LeaveSplit, split, |ws, id, _, cx| {
            if let Some(ix) = ws.find_session(id) {
                ws.leave_split(ix, cx);
            }
        }))
        .separator()
        .item(item("Close pane", ActionId::CloseTerminal, true, |ws, id, window, cx| {
            ws.close_pane(id, window, cx)
        }))
}

/// The View → Layout action for `preset`.
pub(super) fn layout_action(preset: Preset) -> ActionId {
    match preset {
        Preset::Single => ActionId::LayoutSingle,
        Preset::TwoColumns => ActionId::LayoutColumns,
        Preset::TwoRows => ActionId::LayoutRows,
        Preset::Grid => ActionId::LayoutGrid,
    }
}

#[cfg(test)]
mod menu_model_tests {
    use gpui_kit::WeakEntity;

    use super::{PaneFacts, pane_menu};

    fn outline(facts: PaneFacts) -> Vec<String> {
        pane_menu(1, facts, &WeakEntity::new_invalid()).outline()
    }

    /// A lone pane can't be zoomed or left, and a pane with no room below can't split down.
    #[test]
    fn what_a_pane_cannot_do_is_greyed_out() {
        assert_eq!(
            outline(PaneFacts { zoomed: false, split: false, right: true, down: false }),
            [
                "- Split right",
                "- Split down [off]",
                "- Zoom pane [off]",
                "- Move out of split [off]",
                "---",
                "- Close pane"
            ]
        );
        assert_eq!(
            outline(PaneFacts { zoomed: true, split: true, right: true, down: true })[2],
            "- Unzoom pane"
        );
    }
}
