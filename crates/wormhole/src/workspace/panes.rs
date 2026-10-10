//! Split panes on screen: the selected terminal's split drawn as its terminals side by
//! side or stacked, each with a header once split and the dividers over their shared edges. The
//! tree, its geometry and the presets are core's (`wormhole_core::panes`); the model of who is in
//! which split is `splits.rs`. This draws the tree, runs the zoom, the divider drag, the pane menu
//! and the keys between panes, and keeps the selection in step with the keyboard: the keyboard
//! coming into a pane selects its row. A terminal that is in no split draws as it always did
//! (`layout.rs`): no headers, the terminal under the floating title bar.

use std::rc::Rc;

use gpui_kit::component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use wormhole_core::panes::{Axis, Direction, DividerId, Node, PANE_MIN_W, PaneId, Preset, TabPanes};
use wormhole_core::sidebar::SplitId;
use wormhole_core::spoken;

use super::breadcrumb::terminal_glyph;
use super::focus::MenuBuilder;
use super::session::home_dir;
use super::splits::Split;
use super::terminal_drag::{DragOrigin, DraggedTerminal, TerminalDrag, pane_glyph};
use super::*;

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

/// What the divider's tooltip says.
const DIVIDER_HINT: &str = "Drag to resize · double-click to equalize";

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
        notifications::dismiss(id, cx);
        cx.notify();
    }

    /// What a terminal asks of the window: its exited shell's bar has a Restart and a Close.
    pub(super) fn on_terminal_event(
        &mut self,
        id: i64,
        event: TerminalEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.find_session(id) else { return };
        match event {
            TerminalEvent::SummaryChanged => {}
            // The workspace draws the find bar.
            TerminalEvent::FindChanged => cx.notify(),
            TerminalEvent::Restart => self.restart_pane(ix, window, cx),
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
        let pane = self.make_pane(&record, window, cx);
        let focus = pane.terminal.focus_handle(cx);
        self.sessions[ix].pane = pane;
        if ix == self.active {
            window.focus(&focus, cx);
        }
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

    /// Measures the area the panes share (below the title rows, the whole terminal column): what
    /// a divider's drag, ⌥⌘arrows and the room for a split are worked out in.
    pub(super) fn pane_probe(&self, inset: f32, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity().downgrade();
        canvas(
            move |bounds, _, cx| {
                this.update(cx, |this, _| this.pane_area = bounds).ok();
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top(px(inset))
        .bottom_0()
        .left_0()
        .right_0()
        .into_any_element()
    }

    /// The selected terminal's split: each pane with its header, the dividers over their
    /// shared edges, below the title rows (`inset` tall). A zoomed pane alone fills the area.
    pub(super) fn render_panes(&self, inset: f32, cx: &mut Context<Self>) -> AnyElement {
        let Some(split) = self.active_split() else { return div().into_any_element() };
        let order = split.layout.panes();
        let body = match split.layout.zoomed() {
            Some(id) => self.render_pane(split, id, &order, cx),
            None => self.render_node(split, split.layout.tree().root(), &mut Vec::new(), &order, cx),
        };
        div()
            .id("panes")
            .absolute()
            .top(px(inset))
            .bottom_0()
            .left_0()
            .right_0()
            .flex()
            .child(body)
            .into_any_element()
    }

    /// A node of the tree: a pane, or its children along the split's axis, each taking its share
    /// (flex weights, so the shares always fill the split whatever their rounding), with a divider
    /// at the start of every child but the first.
    fn render_node(
        &self,
        split: &Split,
        node: &Node,
        path: &mut Vec<usize>,
        order: &[PaneId],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Node::Split { axis, children } = node else {
            let Node::Leaf(id) = node else { unreachable!("a node is a pane or a split") };
            return self.render_pane(split, *id, order, cx);
        };
        let mut container = div().flex().size_full().min_w_0().min_h_0();
        container = match axis {
            Axis::Row => container.flex_row(),
            Axis::Column => container.flex_col(),
        };
        for (i, child) in children.iter().enumerate() {
            path.push(i);
            let inner = self.render_node(split, &child.node, path, order, cx);
            path.pop();
            let mut cell = div()
                .relative()
                .flex_grow(child.ratio)
                .flex_shrink(1.0)
                .flex_basis(px(0.0))
                .min_w_0()
                .min_h_0()
                .flex()
                .child(inner);
            cell = match axis {
                Axis::Row => cell.h_full(),
                Axis::Column => cell.w_full(),
            };
            if i > 0 {
                let divider = DividerId { path: path.clone(), index: i - 1 };
                cell = cell.child(self.render_divider(split.id, divider, *axis, cx));
            }
            container = container.child(cell);
        }
        container.into_any_element()
    }

    /// One pane: its header and its terminal, the terminal dimmed while another pane has the
    /// keyboard (72 %).
    fn render_pane(&self, split: &Split, id: PaneId, order: &[PaneId], cx: &mut Context<Self>) -> AnyElement {
        let Some(pane) = self.find_session(id).map(|ix| &self.sessions[ix].pane) else {
            return div().into_any_element();
        };
        let focused = split.layout.focused() == id;
        // The pane being dragged by its header stays in its place, lifted.
        let lifted = cx.has_active_drag()
            && self
                .terminal_drag
                .as_ref()
                .is_some_and(|drag| drag.id == id && drag.origin == DragOrigin::PaneHeader);
        let position = order.iter().position(|pane| *pane == id).map_or(1, |at| at + 1);
        let header = self.render_pane_header(split, pane, focused, position, order.len(), cx);
        div()
            .id(("pane", id as u64))
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .when(lifted, |d| d.opacity(style::DRAG_LIFT_OPACITY))
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .when(!focused, |d| d.opacity(style::PANE_DIM))
                    // Cached: window frames the terminal didn't cause reuse its last paint.
                    .child(pane.terminal.clone().cached(StyleRefinement::default().size_full())),
            )
            .into_any_element()
    }

    /// A pane's header: its status slot, what runs in it and where, then Zoom, the pane
    /// menu and Close. The focused pane's header wears the primary line. A click on it gives the
    /// pane the keyboard; a right-click opens the pane menu.
    fn render_pane_header(
        &self,
        split: &Split,
        pane: &Pane,
        focused: bool,
        position: usize,
        of: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let id = pane.id;
        let (status, agent, folder) = {
            let terminal = pane.terminal.read(cx);
            let folder =
                terminal.cwd.as_deref().map(|cwd| wormhole_core::sidebar::display_path(cwd, home_dir()));
            (terminal.status(), terminal.agent.is_some(), folder)
        };
        let name = pane.label(cx);
        let zoomed = split.layout.zoomed() == Some(id);
        let build = self.pane_menu(id, cx);
        let menu = build.clone();
        let zoom_tip = keymap::with_key(if zoomed { "Unzoom pane" } else { "Zoom pane" }, ActionId::ZoomPane);
        // The picture says the terminal's name as its row does.
        let dragged_name =
            self.find_session(id).map_or_else(|| name.clone(), |ix| self.sessions[ix].name(cx));
        let dragged =
            DraggedTerminal::new(id, DragOrigin::PaneHeader, dragged_name, pane_glyph(status, agent));
        let this = cx.weak_entity();
        let close_tip = keymap::with_key("Close pane", ActionId::CloseTerminal);
        style::pane_header(("pane-header", id as u64), ui, focused)
            .role(Role::Group)
            .aria_label(spoken::pane(&name, folder.as_deref(), position, of, focused))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| this.focus_pane(id, window, cx)),
            )
            // Dragged by its header: to the sidebar the terminal leaves the split, onto another
            // pane's edge it moves there.
            .on_drag(dragged, move |dragged, grabbed, _, cx| {
                this.update(cx, |ws, _| {
                    ws.terminal_drag = Some(TerminalDrag::new(dragged.id, dragged.origin))
                })
                .ok();
                cx.new(|_| DraggedTerminal { grabbed, ..dragged.clone() })
            })
            .child(terminal_glyph(id, status, agent, cx))
            // The name keeps its room; the folder gives way first.
            .child(
                div()
                    .flex_none()
                    .max_w(px(style::TITLE_TAB_MAX_W))
                    .truncate()
                    .debug_selector(move || format!("pane-name-{id}"))
                    .child(name),
            )
            .children(folder.map(|folder| {
                div().min_w_0().truncate().font_weight(FontWeight::NORMAL).text_color(ui.faint).child(folder)
            }))
            .child(div().flex_1())
            .child(
                style::icon_button(
                    ("pane-zoom", id as u64),
                    ui,
                    if zoomed { IconName::Minimize } else { IconName::Maximize },
                    zoom_tip,
                )
                .in_row()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.focus_pane(id, window, cx);
                    this.toggle_zoom(cx);
                })),
            )
            .child(
                style::icon_button(("pane-menu", id as u64), ui, IconName::Ellipsis, "Pane menu")
                    .in_row()
                    .dropdown_menu(move |popup, window, cx| menu(popup, window, cx)),
            )
            .child(style::hit_area(
                ("pane-close-hit", id as u64),
                style::CONTROL_H_SM,
                style::close_x(("pane-close", id as u64), ui, style::SelectedFill::Plain),
                |hit| {
                    hit.role(Role::Button)
                        .aria_label("Close pane")
                        .tooltip(move |window, cx| Tooltip::new(close_tip.clone()).build(window, cx))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.close_pane(id, window, cx);
                        }))
                },
            ))
            .context_menu(move |popup, window, cx| build(popup, window, cx))
            .into_any_element()
    }

    /// The divider at the start of a split's child `index + 1` (see `style::pane_divider`): drag
    /// to resize, double-click to equalize, a tooltip that says so.
    fn render_divider(
        &self,
        split: SplitId,
        divider: DividerId,
        axis: Axis,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let dragging =
            self.pane_drag.as_ref().is_some_and(|drag| drag.split == split && drag.divider == divider);
        let look = if dragging {
            style::DividerLook::Dragging
        } else if self.pane_drag.is_none() && self.divider_hot.as_ref() == Some(&divider) {
            style::DividerLook::Hot
        } else {
            style::DividerLook::Rest
        };
        let id: SharedString = format!("pane-divider-{split}-{:?}-{}", divider.path, divider.index).into();
        let (hover, press) = (divider.clone(), divider);
        style::pane_divider(id, ui, axis, look)
            .on_hover(
                cx.listener(move |this, hovered: &bool, _, cx| {
                    this.hover_divider(hover.clone(), *hovered, cx)
                }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.start_divider_drag(split, press.clone(), axis, event, cx)
                }),
            )
            .tooltip(|window, cx| Tooltip::new(DIVIDER_HINT).build(window, cx))
            .into_any_element()
    }

    /// What builds pane `id`'s menu: Split right, Split down, Zoom pane, Move out of
    /// split, then Close pane, each with its key. Every entry acts on this pane, which it gives
    /// the keyboard first.
    fn pane_menu(&self, id: PaneId, cx: &mut Context<Self>) -> MenuBuilder {
        let this = cx.weak_entity();
        let zoomed = self.active_split().is_some_and(|split| split.layout.zoomed() == Some(id));
        let split = self.active_split().is_some();
        let (right, down) = (self.can_split(Axis::Row), self.can_split(Axis::Column));
        Rc::new(move |mut menu: PopupMenu, _: &mut Window, _: &mut Context<PopupMenu>| {
            let item =
                |label: &'static str,
                 action: Box<dyn Action>,
                 enabled: bool,
                 run: fn(&mut Workspace, PaneId, &mut Window, &mut Context<Workspace>)| {
                    let this = this.clone();
                    PopupMenuItem::new(label).action(action).disabled(!enabled).on_click(
                        move |_, window, cx| {
                            this.update(cx, |ws, cx| {
                                ws.focus_pane(id, window, cx);
                                run(ws, id, window, cx);
                            })
                            .ok();
                        },
                    )
                };
            menu = menu
                .item(item("Split right", Box::new(actions::SplitRight), right, |ws, _, window, cx| {
                    ws.split_pane(Axis::Row, window, cx)
                }))
                .item(item("Split down", Box::new(actions::SplitDown), down, |ws, _, window, cx| {
                    ws.split_pane(Axis::Column, window, cx)
                }))
                .item(item(
                    if zoomed { "Unzoom pane" } else { "Zoom pane" },
                    Box::new(actions::ZoomPane),
                    split,
                    |ws, _, _, cx| ws.toggle_zoom(cx),
                ))
                .item(item("Move out of split", Box::new(actions::LeaveSplit), split, |ws, id, _, cx| {
                    if let Some(ix) = ws.find_session(id) {
                        ws.leave_split(ix, cx);
                    }
                }))
                .separator()
                .item(item("Close pane", Box::new(actions::CloseTerminal), true, |ws, id, window, cx| {
                    ws.close_pane(id, window, cx)
                }));
            menu
        })
    }
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
