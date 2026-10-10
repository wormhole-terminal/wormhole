//! The minimap along the pane's right edge (View → Show Minimap): core's `terminal::minimap`
//! reads the lines and says where things go; this paints them and turns clicks and drags into
//! scrolling.

use wormhole_core::terminal::minimap::{BLOCK_H, COLUMN_MAX, LINE_PITCH, MinimapGeometry, WIDTH};

use super::*;

/// Room the minimap takes from the grid.
pub(in crate::terminal_view) const MINIMAP_W: f32 = WIDTH;

impl TerminalView {
    /// The minimap, below the title bar and as tall as the grid.
    pub(super) fn render_minimap(&self, cx: &mut Context<Self>) -> AnyElement {
        let store = cx.global::<SettingsStore>();
        let palette = store.palette.clone();
        let thumb = token(&store.tokens, "scrollbar.thumb");
        let entity = cx.entity();
        let prepaint_state = self.render_state.clone();
        let paint_state = self.render_state.clone();
        let dragging = self.minimap_grab.is_some();
        let weak = entity.downgrade();

        let map = canvas(
            move |bounds, _window, cx| {
                let fits = (f32::from(bounds.size.height) / LINE_PITCH).floor().max(1.0) as usize;
                let mut state = prepaint_state.borrow_mut();
                state.minimap_bounds = bounds;
                let minimap = &mut state.minimap;
                entity.update(cx, |this, _| {
                    if let Some(pty) = &this.pty {
                        pty.minimap(minimap, fits);
                    }
                });
            },
            move |bounds, (), window, _cx| {
                let state = paint_state.borrow();
                let minimap = &state.minimap;
                let column = (WIDTH / minimap.cols.max(1) as f32).min(COLUMN_MAX);
                for (k, runs) in minimap.lines.iter().enumerate() {
                    let y = bounds.origin.y + px(LINE_PITCH * k as f32);
                    for run in runs {
                        let (fg, bg) = palette.cell_colors(&run.cell, false, &minimap.colors);
                        let x = bounds.origin.x + px(column * run.start as f32);
                        let width = px(column * (run.end - run.start) as f32);
                        let block = Bounds::new(point(x, y), size(width, px(BLOCK_H)));
                        window.paint_quad(fill(block, if run.text { fg } else { bg }));
                    }
                }
                let geometry = minimap.geometry;
                if geometry.total > geometry.screen {
                    let (top, height) = geometry.slider();
                    let slider = Bounds::new(
                        point(bounds.origin.x, bounds.origin.y + px(top)),
                        size(bounds.size.width, px(height)),
                    );
                    window.paint_quad(fill(slider, thumb));
                }
                if dragging {
                    drag_listener(weak.clone(), window);
                }
            },
        )
        .size_full();

        div()
            .id("minimap")
            .absolute()
            .top(px(self.top_inset + PADDING_Y))
            .bottom(px(PADDING_Y))
            .right_0()
            .w(px(MINIMAP_W))
            .cursor(CursorStyle::Arrow)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_minimap_down))
            .child(map)
            .into_any_element()
    }

    /// A click on the slider grabs it; anywhere else brings the line there to the middle of the
    /// screen and grabs the slider where it lands.
    fn on_minimap_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // The pane under it would start a selection.
        cx.stop_propagation();
        window.focus(&self.focus_handle, cx);
        let (geometry, y) = self.minimap_at(event.position);
        let (top, height) = geometry.slider();
        let grab = if (top..top + height).contains(&y) {
            y - top
        } else {
            let line = geometry.line_at(y).unwrap_or(geometry.total.saturating_sub(1));
            let view_top = geometry.view_top_centering(line);
            self.scroll_minimap_to(geometry, view_top, cx);
            y - MinimapGeometry { view_top, ..geometry }.slider().0
        };
        self.minimap_grab = Some(grab);
        cx.notify();
    }

    /// The slider follows the pointer while the button is held, wherever the pointer goes.
    fn drag_minimap(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(grab) = self.minimap_grab else { return };
        if event.pressed_button != Some(MouseButton::Left) {
            self.minimap_grab = None;
            cx.notify();
            return;
        }
        let (geometry, y) = self.minimap_at(event.position);
        self.scroll_minimap_to(geometry, geometry.view_top_for_slider(y - grab), cx);
    }

    /// The minimap's geometry as last drawn, and `position`'s height over its top.
    fn minimap_at(&self, position: Point<Pixels>) -> (MinimapGeometry, f32) {
        let state = self.render_state.borrow();
        (state.minimap.geometry, f32::from(position.y - state.minimap_bounds.origin.y))
    }

    fn scroll_minimap_to(&mut self, geometry: MinimapGeometry, view_top: usize, cx: &mut Context<Self>) {
        let Some(pty) = &self.pty else { return };
        let lines = geometry.display_offset(view_top) as i32 - pty.display_offset() as i32;
        if lines != 0 {
            pty.scroll(Scroll::Delta(lines));
            cx.notify();
        }
    }
}

/// While the slider is held, follow the pointer outside the minimap too, and let go wherever
/// the button goes up.
fn drag_listener(entity: WeakEntity<TerminalView>, window: &mut Window) {
    let moved = entity.clone();
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
        if phase == DispatchPhase::Bubble {
            let _ = moved.update(cx, |this, cx| this.drag_minimap(event, cx));
        }
    });
    window.on_mouse_event(move |_: &MouseUpEvent, phase, _window, cx| {
        if phase == DispatchPhase::Bubble {
            let _ = entity.update(cx, |this, cx| {
                this.minimap_grab = None;
                cx.notify();
            });
        }
    });
}
