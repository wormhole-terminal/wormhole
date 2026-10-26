//! Drawing a split: each pane with its header, the dividers over their shared edges, the whole surface.
//! The tree and its geometry are core's; what the panes do when the person acts is `mod.rs`.

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::view::panes::{Axis, DividerId, Node, PaneId};
use wormhole_core::view::sidebar::SplitId;
use wormhole_core::view::spoken;

use super::opening::Stand;
use super::splits::Split;
use crate::platform::{self, Anchor};
use crate::settings::{settings_in, ui_in};
use crate::sidebar::terminal_drag::{DragOrigin, DraggedTerminal, TerminalDrag, pane_glyph};
use crate::style;
use crate::style::home_dir;
use crate::window::Workspace;
use crate::window::session::Pane;
use crate::window::title::breadcrumb::terminal_tile;

/// What the divider's tooltip says.
const DIVIDER_HINT: &str = "Drag to resize · double-click to equalize";

/// How many characters of the folder a pane's header keeps, cut from the front (as many as a
/// sidebar row's second line keeps).
const PANE_PATH_MAX: usize = 38;

impl Workspace {
    /// The selected terminal's split: each pane with its header, the dividers over their
    /// shared edges, the whole surface. A zoomed pane alone fills the area.
    pub(in crate::window) fn render_panes(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(split) = self.active_split() else { return div().into_any_element() };
        // A pane that was just added opens over the layout it replaces.
        if let Some(opening) = self.render_opening(split, cx) {
            return opening;
        }
        let order = split.layout.panes();
        let body = match split.layout.zoomed() {
            Some(id) => self.render_pane(split, id, &order, None, cx),
            None => self.render_node(split, split.layout.tree().root(), &mut Vec::new(), &order, None, cx),
        };
        div().id("panes").absolute().top_0().left_0().size_full().flex().child(body).into_any_element()
    }

    /// A node of the tree: a pane, or its children along the split's axis, each taking its share
    /// (flex weights, so the shares always fill the split whatever their rounding), with a divider
    /// at the start of every child but the first.
    pub(super) fn render_node(
        &self,
        split: &Split,
        node: &Node,
        path: &mut Vec<usize>,
        order: &[PaneId],
        stand: Option<&Stand>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Node::Split { axis, children } = node else {
            let Node::Leaf(id) = node else { unreachable!("a node is a pane or a split") };
            return self.render_pane(split, *id, order, stand, cx);
        };
        let gaps = settings_in(&self.profile, cx).split_gaps;
        let mut container = div().flex().size_full().min_w_0().min_h_0();
        container = match axis {
            Axis::Row => container.flex_row(),
            Axis::Column => container.flex_col(),
        };
        for (i, child) in children.iter().enumerate() {
            path.push(i);
            let mut inner = self.render_node(split, &child.node, path, order, stand, cx);
            path.pop();
            let gap = if i > 0 && gaps { px(style::PANE_GAP) } else { px(0.0) };
            // The pane that is being split gives way to the new one: its box is the room the edge leaves it.
            if let Some(stand) =
                stand.filter(|stand| matches!(child.node, Node::Leaf(id) if stand.is_split(id)))
            {
                let lead = f32::from(gap);
                inner = self.kept_box(
                    inner,
                    stand,
                    match axis {
                        Axis::Row => (lead, 0.0),
                        Axis::Column => (0.0, lead),
                    },
                );
            }
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
    pub(super) fn render_pane(
        &self,
        split: &Split,
        id: PaneId,
        order: &[PaneId],
        stand: Option<&Stand>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(pane) = self.find_session(id).map(|ix| &self.sessions[ix].pane) else {
            return div().into_any_element();
        };
        // While a split opens the keyboard is already in the new pane, which the layout before it does not know.
        let focused = stand.map_or_else(|| split.layout.focused() == id, |stand| stand.focused(id));
        // A pane that is being split keeps the terminal it had, whole, from its top left (`panes::opening`).
        let held = stand.filter(|stand| stand.is_split(id)).map(Stand::hold);
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
            // Which layer paints the pane's colour, for the tests: its own with surfaces, the column's without.
            .debug_selector(move || if gaps { format!("pane-coat-{id}") } else { format!("pane-{id}") })
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
                    .when(held.is_some(), |d| d.overflow_hidden())
                    // Cached (`window::cached`): window frames the terminal didn't cause reuse its last paint.
                    .child({
                        let terminal = crate::window::cached::view(
                            &pane.terminal,
                            StyleRefinement::default().size_full(),
                        );
                        match held {
                            Some((w, h)) => {
                                div().flex_none().w(px(w)).h(px(h)).child(terminal).into_any_element()
                            }
                            None => terminal,
                        }
                    }),
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
            let summary = &pane.summary;
            // Cut from the front, as the sidebar's rows do: the end tells two folders apart.
            let folder = wormhole_core::view::sidebar::wording::place_detail(
                summary.cwd.as_deref(),
                home_dir(),
                PANE_PATH_MAX,
            );
            (summary.status, summary.source.told(), summary.agent, folder)
        };
        // The terminal's own name, as its sidebar row says it (a rename shows here too).
        let name = self.find_session(id).map_or_else(|| pane.label(), |ix| self.sessions[ix].name());
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
            .on_drag(dragged, move |dragged, grabbed, window, cx| {
                this.update(cx, |ws, cx| {
                    ws.terminal_drag = Some(TerminalDrag::new(dragged.id, dragged.origin));
                    cx.notify();
                })
                .ok();
                let source = Some(window.window_handle().window_id());
                cx.new(|_| DraggedTerminal { grabbed, source, ..dragged.clone() })
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
                                    platform::show_menu(menu, menu_anchor.below(), window, cx);
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
                        // Zoomed by the keys or by this button, the old corners shrink away as the new ones grow in.
                        .motion(style::IconMotion::Swap)
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
                    platform::show_menu(menu, event.position, window, cx);
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
}
