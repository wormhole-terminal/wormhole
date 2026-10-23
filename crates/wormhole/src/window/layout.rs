//! Window layout: the side panels (toggle, fit) and the root render. The title rows are in
//! `chrome`, the resize handles in `resize`.
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::layout::{Columns, Edge, Panes};

use crate::app_state::app_mut;
use crate::settings::{settings_in, store_in, ui_in};
use crate::style;
use crate::window::Workspace;
use crate::window::panes;
use crate::window::resize::edge_rule;

impl Workspace {
    pub(crate) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        // Under tabs there is no sidebar to show or hide.
        if self.sidebar_feed.tabs_on {
            return;
        }
        let before = self.columns().sidebar;
        self.panes.toggle_sidebar();
        self.start_slide(Edge::Sidebar, before, self.columns().sidebar, cx);
        self.remember_panes(cx);
        self.mark_dirty(cx);
    }

    /// Tell the app how the columns stand, so a window opened later with no layout of its own
    /// starts that way. Called where the person changes them, not on every frame.
    pub(super) fn remember_panes(&self, cx: &mut Context<Self>) {
        app_mut(cx).remember_panes(&self.panes);
    }

    /// Whether the right panel is on screen: wanted and roomy. Auto-hiding leaves it wanted,
    /// so widening the window brings the panel back unless the user had closed it.
    pub(super) fn notes_visible(&self) -> bool {
        self.shown_panes().panel_visible(self.window_width)
    }

    /// The columns as they are drawn: under tabs there is no sidebar, whatever it would be.
    fn shown_panes(&self) -> Panes {
        let mut panes = self.panes;
        panes.sidebar_open &= !self.sidebar_feed.tabs_on;
        panes
    }

    /// Makes the panel appear. With no room for it the sidebar steps aside (the
    /// user asked for the panel just now, so it wins); ⌃⌘S brings the sidebar back
    /// and, if that leaves no room, hides the panel again.
    pub(super) fn reveal_notes(&mut self) {
        self.panes.reveal_panel(self.window_width);
    }

    pub(crate) fn toggle_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let before = self.columns().panel;
        if !self.panes.toggle_panel(self.window_width) {
            window.focus(&self.active_terminal().focus_handle(cx), cx);
        }
        self.start_slide(Edge::Panel, before, self.columns().panel, cx);
        self.remember_panes(cx);
        self.mark_dirty(cx);
    }

    /// The three columns' widths as of the last frame.
    pub(crate) fn columns(&self) -> Columns {
        self.shown_panes().columns(self.window_width)
    }

    /// Measures the workspace's column. With the debug Inspector open, GPUI lays the root out in
    /// the window minus the Inspector's panel, and nothing but the root's own bounds says so
    /// (`viewport_size` is the whole window): the panel's handle, its fit and a drag's extent
    /// would count the Inspector as part of the window. A frame laid out with another share
    /// than it measures is followed by one more (`beside`).
    fn column_probe(&self, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity().downgrade();
        let glass = self.glass.clone();
        canvas(
            move |bounds, window, cx| {
                // The last probe of the frame: every region that wants glass has said so by now.
                glass.commit();
                let beside = f32::from(window.viewport_size().width - bounds.size.width);
                let changed = this.update(cx, |this, _| {
                    let changed = (this.beside - beside).abs() > 0.5;
                    this.beside = beside;
                    changed
                });
                if changed.unwrap_or(false) {
                    window.on_next_frame(|window, _| window.refresh());
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .bottom_0()
        .left_0()
        .right_0()
        .into_any_element()
    }

    /// The focused terminal's find bar, floating at the terminal's top right, under the
    /// title bar (in a split under its pane's header) and lined up with the grid's right edge.
    fn render_find_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let terminal = self.active_terminal();
        let terminal = terminal.read(cx);
        let bar = terminal.find_bar()?;
        let right = px(crate::terminal_view::TerminalView::right_inset(&self.profile, cx));
        let (top, right) = match self.active_split() {
            Some(split) => {
                let (w, h) = (f32::from(self.pane_area.size.width), f32::from(self.pane_area.size.height));
                let rect = split.layout.shown(w, h).into_iter().find(|(id, _)| *id == split.layout.focused());
                match rect {
                    Some((_, rect)) => {
                        let header = style::pane_header_h(settings_in(&self.profile, cx).split_gaps);
                        (rect.y + header, px(w - rect.right()) + right)
                    }
                    None => (0.0, right),
                }
            }
            None => (0.0, right),
        };
        Some(div().absolute().top(px(top)).mt_2().right(right).child(bar).into_any_element())
    }

    // ---------------------------------------------------------------- keys
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The workspace's own column: the window less what lies beside it (`column_probe`).
        self.window_width = f32::from(window.viewport_size().width) - self.beside;
        self.sync_tab_layout(window, cx);
        // A drag that ended outside every drop target never heard its drop.
        if (self.terminal_drag.is_some() || self.group_drag.is_some()) && !cx.has_active_drag() {
            self.terminal_drag = None;
            self.group_drag = None;
        }
        let notes_visible = self.notes_visible();
        let columns = self.columns();
        // What a feature lays over the terminal (a diff), built for this frame.
        let cover = self.render_cover(window, cx);
        let covered = cover.is_some();
        let ui = ui_in(&self.profile, cx);
        let session = self.active_session();
        let terminal = session.terminal().clone();
        let split = self.active_split().is_some();
        let gaps = settings_in(&self.profile, cx).split_gaps && !covered;
        // One geometry for both looks: the title bar has its own row and the terminal's rounded surface
        // lies below it, so nothing scrolls under the bar. Glass only changes the material.
        let lifted = self.glass_on(cx);
        let titlebar = self.render_terminal_title_row(window, cx);
        let [sidebar_toggle, panel_toggle] = self.render_window_controls(window, cx);
        self.status_bar_drawn = self.status_bar(cx);
        let status_bar = self.status_bar_drawn.as_ref().map(|bar| self.render_status_bar(bar, window, cx));
        let sidebar_glass = lifted;
        let tabs = self.sidebar_feed.tabs_on;
        let top_tabs = if tabs { self.render_tab_bar(window, cx) } else { None };
        // One terminal and no group to switch to: no tab bar, as in macOS Terminal.
        let tabs_shown = top_tabs.is_some();

        // GPUI Kit's root sets the interface font and the rem size from the shown profile's
        // theme; an odd window overrides both with its own profile's.
        let own_font = self.odd_profile(cx).map(|_| {
            let fonts = &store_in(&self.profile, cx).fonts;
            window.set_rem_size(crate::theme_bridge::rem_size(fonts));
            SharedString::from(fonts.ui_family.clone())
        });
        let root = div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            // Each region paints its own surface (translucent over the native blur).
            .text_color(ui.text)
            .when_some(own_font, |d, family| d.font_family(family))
            .key_context("Workspace");
        // While the intro window is open the window's actions wait: its keys go to it.
        let intro_open = self.first_run_open(cx);
        let root = if intro_open { root } else { self.with_actions(root, cx) };
        // What the intro window needs of this one is told before it is drawn.
        self.push_first_run(cx);
        let intro = self.first_run.clone();
        root
            // The intro window holds the keyboard while it is open.
            .capture_key_down(move |event: &KeyDownEvent, window, cx| {
                intro.update(cx, |intro, cx| intro.intro_keys(event, window, cx))
            })
            .capture_key_down(cx.listener(Self::on_key_down))
            .on_modifiers_changed(cx.listener(Self::on_modifiers))
            .on_mouse_move(cx.listener(Self::on_resize_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_resize_end))
            .on_mouse_move(cx.listener(Self::on_divider_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_divider_end))
            .when(self.resizing.is_some(), |d| d.cursor(CursorStyle::ResizeLeftRight))
            .when_some(self.pane_drag.as_ref().map(panes::PaneDrag::cursor), |d, cursor| d.cursor(cursor))
            // The panes, above the status bar when it is shown and to the window's bottom when not.
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .when(self.sidebar_shown() && self.in_flow(Edge::Sidebar), |d| {
                        d.child(self.render_sidebar(columns.sidebar, window, cx))
                    })
                    .child({
                        let surface = div()
                            .relative()
                            .flex_1()
                            // Never squeezed below the terminal's minimum: the panels yield
                            // first (core `Panes::columns`).
                            .min_w(px(crate::terminal_view::MIN_WIDTH))
                            // No fill of its own: the column has the terminal's colour, and a second
                            // coat of a translucent one would draw the surface as a darker box. A split
                            // with gaps: each pane is the surface, the gaps show the window.
                            .children(cover)
                            // Cached: window frames the terminal didn't cause reuse its last
                            // paint. A split's panes are drawn with their headers below the
                            // title rows.
                            .when(!covered && !split, |d| {
                                d.child(terminal.cached(StyleRefinement::default().size_full()))
                            })
                            .when(!covered && split, |d| d.child(self.render_panes(cx)))
                            .when(!covered, |d| d.child(self.pane_probe(cx)))
                            // While a terminal is dragged, over the panes (the terminal swallows the mouse).
                            .when(!covered, |d| d.children(self.render_drop_overlay(cx)))
                            .children(self.render_find_bar(cx));
                        // The terminal is a rounded surface below the title row, its own inset from the
                        // column's sides and bottom; with the grid's padding inside it the text stands on the
                        // window's 12 pt edge and lines up with the traffic lights. The margin also keeps a
                        // program that paints every cell (a full-screen app's rectangle of colour) clear of
                        // the window's own corners.
                        let gap = px(style::SURFACE_INSET);
                        let column = div()
                            .relative()
                            .flex_1()
                            .min_w(px(crate::terminal_view::MIN_WIDTH))
                            .h_full()
                            .flex()
                            .flex_col()
                            // The terminal's colour from the title row down, as in 1.1, so the surface
                            // reads as the terminal itself, not a box on the window. A split with gaps
                            // keeps the window's colour, which the gaps show.
                            .when(!(split && gaps), |d| d.bg(ui.background))
                            .child(
                                surface
                                    .min_h_0()
                                    // Terminals as tabs: their bar lies between the title row and the surface.
                                    .mt(px(crate::style::TITLE_ROW_H
                                        + if tabs_shown { crate::sidebar::TAB_ROW_H } else { 0.0 }))
                                    .mx(gap)
                                    .mb(gap)
                                    .when(!(split && gaps), |d| d.rounded(ui.radius_lg).overflow_hidden()),
                            )
                            .child(titlebar)
                            .children(top_tabs);
                        self.carry_with_slide(column)
                    })
                    .when(notes_visible && self.in_flow(Edge::Panel), |d| {
                        d.child(self.render_panel(columns.panel, window, cx))
                    })
                    // A column sliding in or out lies over the terminal.
                    .children(self.render_slide(window, cx))
                    // The dividers lie over the columns' edges and their handles over them.
                    .when(columns.sidebar > 0.0 && self.in_flow(Edge::Sidebar), |d| {
                        // Glass floats inset, so no hairline runs along the column's edge.
                        d.when(!sidebar_glass, |d| d.children(edge_rule(Edge::Sidebar, columns.sidebar, ui)))
                            .child(self.resize_handle(Edge::Sidebar, columns.sidebar, cx))
                    })
                    .when(columns.panel > 0.0 && self.in_flow(Edge::Panel), |d| {
                        let x = self.window_width - columns.panel;
                        d.when(!sidebar_glass, |d| d.children(edge_rule(Edge::Panel, x, ui)))
                            .child(self.resize_handle(Edge::Panel, x, cx))
                    }),
            )
            .children(status_bar)
            .when(!tabs, |d| d.child(sidebar_toggle))
            .child(panel_toggle)
            .children(self.render_profile_edge(cx))
            .children(self.resize_readout(cx))
            .child(self.render_focus_holders(cx))
            .children(self.render_overlay())
            // GPUI's own menu and popover, where a native one can't open (`style::menu_layer`).
            .child(crate::style::menu_layer::layer(window, cx))
            .child(crate::style::popover_layer::layer(window, cx))
            .when(intro_open, |d| d.child(self.first_run.clone()))
            .child(self.column_probe(cx))
    }
}
