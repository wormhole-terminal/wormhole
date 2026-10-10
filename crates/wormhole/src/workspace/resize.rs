//! The edges between the side panels and the terminal: a hairline drawn over each edge, and
//! resizing the panels from it. An 8 pt handle straddles each edge; after a short hover
//! its 2 pt line lights up, while dragging it is solid and a
//! readout beside the pointer says the width and why it stopped. A double-click puts the
//! default width back, Esc puts back where the drag began. The rules are core's
//! (`wormhole_core::layout`).
use super::*;
use wormhole_core::layout::Resize;

/// A drag on a panel's edge, from the press to the release.
pub(super) struct ResizeDrag {
    pub edge: Edge,
    /// The panels as they were when the button went down: Esc goes back to them.
    before: Panes,
    /// Where the pointer is, for the readout beside it.
    pointer: Point<Pixels>,
    /// Where the drag has the panel now; `None` until the pointer moves.
    at: Option<Resize>,
}

/// The handle's line, centred on the edge (it covers the hairline beside it).
const LINE_W: f32 = 2.0;
/// How far the readout sits from the pointer.
const READOUT_OFFSET: f32 = 12.0;

/// The hairline on `edge`, which is `x` from the window's left. It is drawn over the columns
/// instead of being a border of one of them, so neither gives up a pixel to it: the sidebar is
/// exactly its width (its right-hand items end 16 pt inside, on x 334 of 350) and the panel
/// exactly its own. It lies on the terminal's side of the edge.
#[track_caller]
pub(super) fn edge_rule(edge: Edge, x: f32, ui: UiColors) -> Div {
    let left = match edge {
        Edge::Sidebar => x,
        Edge::Panel => x - f32::from(ui.border_width),
    };
    div().absolute().top_0().bottom_0().left(px(left)).w(ui.border_width).bg(ui.border)
}

impl Workspace {
    fn start_resize(&mut self, edge: Edge, event: &MouseDownEvent, cx: &mut Context<Self>) {
        cx.stop_propagation();
        if event.click_count >= 2 {
            self.panes.reset(edge);
            self.resizing = None;
            self.remember_panes(cx);
            self.mark_dirty(cx);
            return;
        }
        self.resizing = Some(ResizeDrag { edge, before: self.panes, pointer: event.position, at: None });
        cx.notify();
    }

    pub(super) fn on_resize_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = &mut self.resizing else { return };
        if event.pressed_button != Some(MouseButton::Left) {
            // Released outside the window.
            self.resizing = None;
            self.remember_panes(cx);
            self.mark_dirty(cx);
            return;
        }
        // The workspace's column, not the viewport: the debug Inspector may take a share.
        let window_width = self.window_width;
        let x = f32::from(event.position.x);
        let extent = match drag.edge {
            Edge::Sidebar => x,
            Edge::Panel => window_width - x,
        };
        drag.pointer = event.position;
        // Past the minimum it sticks, then snaps closed; dragging back reopens.
        drag.at = Some(self.panes.drag(drag.edge, extent, window_width));
        cx.notify();
    }

    pub(super) fn on_resize_end(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.resizing.take().is_some() {
            self.remember_panes(cx);
            self.mark_dirty(cx);
        }
    }

    /// Esc while dragging: the panels go back to where the drag began. Returns whether a drag
    /// was going on.
    pub(super) fn cancel_resize(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(drag) = self.resizing.take() else { return false };
        self.panes = drag.before;
        cx.notify();
        true
    }

    /// The pointer entered or left a handle: its line lights up only after a short hover, so
    /// crossing the edge on the way somewhere else doesn't flash it.
    fn hover_handle(&mut self, edge: Edge, hovered: bool, cx: &mut Context<Self>) {
        if !hovered {
            self.resize_hover = None;
            if self.resize_hot.take().is_some() {
                cx.notify();
            }
            return;
        }
        self.resize_hover = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(style::motion::RESIZE_HANDLE_HOVER_DELAY).await;
            this.update(cx, |this, cx| {
                this.resize_hot = Some(edge);
                cx.notify();
            })
            .ok();
        }));
    }

    /// The grab area on `edge`, at `x` from the window's left edge.
    pub(super) fn resize_handle(&self, edge: Edge, x: f32, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let dragging = self.resizing.as_ref().is_some_and(|drag| drag.edge == edge);
        let line = if dragging {
            Some(ui.resize_active)
        } else {
            (self.resizing.is_none() && self.resize_hot == Some(edge)).then_some(ui.resize_hover)
        };
        let id = match edge {
            Edge::Sidebar => "sidebar-resize",
            Edge::Panel => "panel-resize",
        };
        div()
            .id(id)
            // The view tests measure where it sits.
            .debug_selector(|| id.to_string())
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(x - style::RESIZE_HANDLE_W / 2.0))
            .w(px(style::RESIZE_HANDLE_W))
            .cursor(CursorStyle::ResizeLeftRight)
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| this.hover_handle(edge, *hovered, cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| this.start_resize(edge, event, cx)),
            )
            .children(line.map(|color| {
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px((style::RESIZE_HANDLE_W - LINE_W) / 2.0))
                    .w(px(LINE_W))
                    .bg(color)
            }))
            .into_any_element()
    }

    /// The width readout beside the pointer while a drag is on.
    pub(super) fn resize_readout(&self, cx: &App) -> Option<AnyElement> {
        let drag = self.resizing.as_ref()?;
        let at = drag.at?;
        Some(
            div()
                .absolute()
                .left(drag.pointer.x + px(READOUT_OFFSET))
                .top(drag.pointer.y - px(READOUT_OFFSET))
                .child(style::readout(ui(cx), at.readout(drag.edge)))
                .into_any_element(),
        )
    }
}
