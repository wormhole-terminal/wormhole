//! Split panes on screen: the selected terminal's split drawn as its terminals side by
//! side or stacked, each with a header once split and the dividers over their shared edges. The
//! tree, its geometry and the presets are core's (`wormhole_core::panes`); the model of who is in
//! which split is `splits.rs`. This draws the tree, runs the zoom, the divider drag, the pane menu
//! and the keys between panes, and keeps the selection in step with the keyboard: the keyboard
//! coming into a pane selects its row. A terminal that is in no split draws as it always did
//! (`layout.rs`): no headers, the terminal under the floating title bar.

use gpui_kit::component::tooltip::Tooltip;
use wormhole_core::panes::{Axis, Direction, DividerId, Node, PANE_MIN_W, PaneId, Preset, TabPanes};
use wormhole_core::sidebar::SplitId;
use wormhole_core::spoken;

use super::breadcrumb::terminal_tile;
use super::session::home_dir;
use super::splits::Split;
use super::terminal_drag::{DragOrigin, DraggedTerminal, TerminalDrag, pane_glyph};
use super::*;
use crate::menubar::model::KeyEquivalent;
use crate::native::{self, Anchor, Item, Menu};

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
/// How many characters of the folder a pane's header keeps, cut from the front (as many as a
/// sidebar row's second line keeps).
const PANE_PATH_MAX: usize = 38;

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
            // Its row names the host it has now (none, after a fallback or a lost host).
            TerminalEvent::HostChanged => self.dirty = true,
            // Not left for the next minute's save: what was cleared may have been a secret. Only
            // the copy on disk: a hosted terminal's session host keeps its own history and paints it
            // again when the app attaches (clearing it there is the host's protocol to change).
            TerminalEvent::Cleared => {
                if let Some(store) = app(cx).store_of(&self.profile)
                    && let Err(err) = store.clear_scrollback(id)
                {
                    tracing::warn!("clearing terminal {id}'s saved text failed: {err:#}");
                }
            }
            // The workspace draws the find bar.
            TerminalEvent::FindChanged => cx.notify(),
            TerminalEvent::Restart => self.restart_pane(ix, window, cx),
            // A program (with `allow_window_ops`) asks the window for something: it is read as
            // being about the pane that asked.
            TerminalEvent::WindowOp(op) => {
                let terminal = self.sessions[ix].terminal().clone();
                let pane = terminal.read(cx).pane_size();
                let done = crate::window_ops::apply(op, pane, window, cx);
                terminal.update(cx, |terminal, cx| terminal.window_op_done(done, cx));
            }
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

    /// The selected terminal's split: each pane with its header, the dividers over their
    /// shared edges, the whole surface. A zoomed pane alone fills the area.
    pub(super) fn render_panes(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(split) = self.active_split() else { return div().into_any_element() };
        let order = split.layout.panes();
        let body = match split.layout.zoomed() {
            Some(id) => self.render_pane(split, id, &order, cx),
            None => self.render_node(split, split.layout.tree().root(), &mut Vec::new(), &order, cx),
        };
        div().id("panes").absolute().top_0().left_0().size_full().flex().child(body).into_any_element()
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
        let gaps = settings_in(&self.profile, cx).split_gaps;
        let mut container = div().flex().size_full().min_w_0().min_h_0();
        container = match axis {
            Axis::Row => container.flex_row(),
            Axis::Column => container.flex_col(),
        };
        for (i, child) in children.iter().enumerate() {
            path.push(i);
            let inner = self.render_node(split, &child.node, path, order, cx);
            path.pop();
            let gap = if i > 0 && gaps { px(style::PANE_GAP) } else { px(0.0) };
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
                Axis::Row => cell.h_full().pl(gap),
                Axis::Column => cell.w_full().pt(gap),
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
        let gaps = settings_in(&self.profile, cx).split_gaps;
        let ui = ui_in(&self.profile, cx);
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
            // Each pane its own surface, as the single terminal's.
            .when(gaps, |d| d.bg(ui.background).rounded(ui.radius_lg))
            .when(lifted, |d| d.opacity(style::DRAG_LIFT_OPACITY))
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    // A pane that is its own box keeps its text off the box's edge, under the
                    // header's tile.
                    .when(gaps, |d| d.px(px(style::PANE_TEXT_PAD)).pb(px(style::PANE_TEXT_PAD)))
                    .when(!focused, |d| d.opacity(style::PANE_DIM))
                    // Cached: window frames the terminal didn't cause reuse its last paint.
                    .child(pane.terminal.clone().cached(StyleRefinement::default().size_full())),
            )
            .when(gaps, |d| d.child(style::pane_outline(ui, focused)))
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
        let ui = ui_in(&self.profile, cx);
        let gaps = settings_in(&self.profile, cx).split_gaps;
        let id = pane.id;
        let (status, told, agent, folder) = {
            let terminal = pane.terminal.read(cx);
            // Cut from the front, as the sidebar's rows do: the end tells two folders apart.
            let folder = wormhole_core::sidebar::text::place_detail(
                terminal.cwd.as_deref(),
                home_dir(),
                PANE_PATH_MAX,
            );
            (terminal.status(), terminal.source().told(), terminal.agent, folder)
        };
        // The terminal's own name, as its sidebar row says it (a rename shows here too).
        let name = self.find_session(id).map_or_else(|| pane.label(cx), |ix| self.sessions[ix].name(cx));
        let zoomed = split.layout.zoomed() == Some(id);
        let menu_anchor = Anchor::default();
        let zoom_tip = keymap::with_key(if zoomed { "Unzoom pane" } else { "Zoom pane" }, ActionId::ZoomPane);
        let dragged =
            DraggedTerminal::new(id, DragOrigin::PaneHeader, name.clone(), pane_glyph(status, told));
        let this = cx.weak_entity();
        let close_tip = keymap::with_key("Close pane", ActionId::CloseTerminal);
        style::pane_header(("pane-header", id as u64), ui, focused, gaps)
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
            .child(terminal_tile(("pane-tile", id as u64), agent, told, status, ui, cx))
            // The folder gives way first (it only takes the room left over), then the name; the
            // buttons never do, so a narrow pane can still be zoomed and closed.
            .child(
                div()
                    .min_w_0()
                    .max_w(px(style::TITLE_TAB_MAX_W))
                    .truncate()
                    .debug_selector(move || format!("pane-name-{id}"))
                    .child(name),
            )
            .child(match folder {
                Some(folder) => div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_weight(FontWeight::NORMAL)
                    .text_color(ui.faint)
                    .child(folder),
                None => div().flex_1(),
            })
            // The buttons from the right: in a pane too narrow for all three, Zoom goes first and
            // the menu next (both also on the right-click), never the close button.
            .child(
                div()
                    .flex()
                    .flex_row_reverse()
                    .items_center()
                    .min_w_0()
                    .overflow_hidden()
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
                    .child(
                        div().relative().flex_none().child(menu_anchor.probe()).child(
                            style::icon_button(("pane-menu", id as u64), ui, IconName::Ellipsis, "Pane menu")
                                .in_row()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let menu = this.pane_menu(id, cx);
                                    native::show_menu(menu, menu_anchor.below(), window, cx);
                                })),
                        ),
                    )
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
                    ),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    let menu = this.pane_menu(id, cx);
                    native::show_menu(menu, event.position, window, cx);
                }),
            )
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
        let ui = ui_in(&self.profile, cx);
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
        let gaps = settings_in(&self.profile, cx).split_gaps;
        style::pane_divider(id, ui, axis, look, gaps)
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
            .on_select(native::run_on(this, move |ws, window, cx| {
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
