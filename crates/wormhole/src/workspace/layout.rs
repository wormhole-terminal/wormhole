//! Window layout: the side panels (toggle, fit) and the root render. The title rows are in
//! `chrome`, the resize handles in `resize`.
use super::resize::edge_rule;
use super::*;

impl Workspace {
    pub(crate) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
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
        self.panes.panel_visible(self.window_width)
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
    pub(super) fn columns(&self) -> Columns {
        self.panes.columns(self.window_width)
    }

    /// Measures the workspace's column. With the debug Inspector open, GPUI lays the root out in
    /// the window minus the Inspector's panel, and nothing but the root's own bounds says so
    /// (`viewport_size` is the whole window): the panel's handle, its fit and a drag's extent
    /// would count the Inspector as part of the window. A frame laid out with another share
    /// than it measures is followed by one more (`beside`).
    fn column_probe(&self, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity().downgrade();
        canvas(
            move |bounds, window, cx| {
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
    fn render_find_bar(&self, inset: f32, cx: &mut Context<Self>) -> Option<AnyElement> {
        let terminal = self.active_terminal();
        let terminal = terminal.read(cx);
        let bar = terminal.find_bar()?;
        let right = px(crate::terminal_view::TerminalView::right_inset(cx));
        let (top, right) = match self.active_split() {
            Some(split) => {
                let (w, h) = (f32::from(self.pane_area.size.width), f32::from(self.pane_area.size.height));
                let rect = split.layout.shown(w, h).into_iter().find(|(id, _)| *id == split.layout.focused());
                match rect {
                    Some((_, rect)) => (inset + rect.y + style::PANE_HEADER_H, px(w - rect.right()) + right),
                    None => (inset, right),
                }
            }
            None => (terminal.top_inset(), right),
        };
        Some(div().absolute().top(px(top)).mt_2().right(right).child(bar).into_any_element())
    }

    // ---------------------------------------------------------------- keys
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The workspace's own column: the window less what lies beside it (`column_probe`).
        self.window_width = f32::from(window.viewport_size().width) - self.beside;
        // A drag that ended outside every drop target never heard its drop.
        if self.terminal_drag.is_some() && !cx.has_active_drag() {
            self.terminal_drag = None;
        }
        if self.picking_folder.is_some() && !self.folder_picker.read(cx).is_open() {
            self.picking_folder = None;
        }
        let notes_visible = self.notes_visible();
        let columns = self.columns();
        if notes_visible {
            // The panel's long lists are virtualized; bring their rows up to date first.
            match self.right_tab {
                RightTab::Changes => self.sync_changes_list(cx),
                RightTab::History => self.sync_history_list(),
                RightTab::Notes => self.sync_notes_list(cx),
                RightTab::Project => self.sync_folder_picker(window, cx),
                RightTab::Usage => {}
            }
        }
        let ui = ui(cx);
        let session = self.active_session();
        let terminal = session.terminal().clone();
        let split = self.active_split().is_some();
        // The diff sits in its own card below the bar, so nothing scrolls under it then; a split's
        // panes start below the bar, with headers of their own.
        let scrolled = self.diff_view.is_none() && !split && terminal.read(cx).has_content_above();
        let inset = crate::terminal_view::TOP_INSET;
        // A pane of a split has its header over it instead of the title bar.
        let pane_inset = if split { 0.0 } else { inset };
        for ix in self.on_screen_sessions() {
            self.sessions[ix].terminal().update(cx, |terminal, cx| terminal.set_top_inset(pane_inset, cx));
        }
        let titlebar = self.render_terminal_title_row(scrolled, window, cx);
        let [sidebar_toggle, panel_toggle] = self.render_window_controls(window, cx);
        self.status_bar_drawn = self.status_bar(cx);
        let status_bar = self.status_bar_drawn.as_ref().map(|bar| self.render_status_bar(bar, window, cx));

        let root = div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            // Each region paints its own surface (translucent over the native blur).
            .text_color(ui.text)
            .key_context("Workspace");
        // While the intro window is open the window's actions wait: its keys go to it.
        let root = if self.first_run.is_open() { root } else { self.with_actions(root, cx) };
        root
            // The intro window holds the keyboard while it is open.
            .capture_key_down(cx.listener(Self::intro_keys))
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
                    .when(self.panes.sidebar_open && self.in_flow(Edge::Sidebar), |d| {
                        d.child(self.render_sidebar(columns.sidebar, window, cx))
                    })
                    .child(
                        self.carry_with_slide(
                            div()
                                .relative()
                                .flex_1()
                                // Never squeezed below the terminal's minimum: the panels yield
                                // first (core `Panes::columns`).
                                .min_w(px(crate::terminal_view::MIN_WIDTH))
                                .h_full()
                                .bg(ui.background)
                                .children(self.render_diff(cx))
                                // Cached: window frames the terminal didn't cause reuse its last
                                // paint. A split's panes are drawn with their headers below the
                                // title rows.
                                .when(self.diff_view.is_none() && !split, |d| {
                                    d.child(terminal.cached(StyleRefinement::default().size_full()))
                                })
                                .when(self.diff_view.is_none() && split, |d| {
                                    d.child(self.render_panes(inset, cx))
                                })
                                .when(self.diff_view.is_none(), |d| d.child(self.pane_probe(inset, cx)))
                                // While a terminal is dragged, over the panes (the terminal swallows the mouse).
                                .when(self.diff_view.is_none(), |d| {
                                    d.children(self.render_drop_overlay(inset, cx))
                                })
                                .child(titlebar)
                                .children(self.render_find_bar(inset, cx)),
                        ),
                    )
                    .when(notes_visible && self.in_flow(Edge::Panel), |d| {
                        d.child(self.render_notes_panel(columns.panel, window, cx))
                    })
                    // A column sliding in or out lies over the terminal.
                    .children(self.render_slide(window, cx))
                    // The hairlines lie over the columns' edges and their handles over them.
                    .when(columns.sidebar > 0.0 && self.in_flow(Edge::Sidebar), |d| {
                        d.child(edge_rule(Edge::Sidebar, columns.sidebar, ui)).child(self.resize_handle(
                            Edge::Sidebar,
                            columns.sidebar,
                            cx,
                        ))
                    })
                    .when(columns.panel > 0.0 && self.in_flow(Edge::Panel), |d| {
                        let x = self.window_width - columns.panel;
                        d.child(edge_rule(Edge::Panel, x, ui)).child(self.resize_handle(Edge::Panel, x, cx))
                    }),
            )
            .children(status_bar)
            .child(sidebar_toggle)
            .child(panel_toggle)
            .children(self.resize_readout(cx))
            .child(self.render_focus_holders(cx))
            .children(self.render_overlay())
            .children(self.render_intro(window, cx))
            .child(self.column_probe(cx))
    }
}
