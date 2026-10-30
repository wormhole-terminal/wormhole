//! The Studio window's shell: the title row, the notice, and the three columns, each a cached view
//! of its own. A view that is not notified is not drawn again (its last frame is reused), so
//! the shell can be redrawn without the gallery, the list and the editor, and each of those
//! alone.

use gpui_kit::*;

use super::gallery::Area;
use super::token_editor::EDITOR_W;
use super::token_list::LIST_W;
use super::{StudioWindow, ui};
use crate::settings::settings;
use crate::style;

impl StudioWindow {
    /// The gallery's filter, which narrows the token list to the same part of the app.
    pub(super) fn set_area(&mut self, area: Area, cx: &mut Context<Self>) {
        self.area = area;
        cx.notify();
    }
}

impl Render for StudioWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let translucent = settings(cx).studio_translucent;
        self.blur.set_enabled(style::wants_blur(settings(cx), translucent));
        let surfaces = style::window_surfaces(cx, translucent);
        div()
            .id("studio-window")
            .track_focus(&self.focus_handle)
            .key_context("StudioWindow")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                crate::style::native_focus::keyboard(window);
                this.on_key(event, window, cx);
            }))
            .size_full()
            .flex()
            .flex_col()
            .text_color(ui.text)
            .child(self.render_title_bar(surfaces.panel, style::lights_end(window), cx))
            .children(self.render_notice(surfaces.background, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    // Each region paints its own surface (no stacked translucent layers).
                    .child(
                        self.list
                            .clone()
                            .cached(StyleRefinement::default().w(px(LIST_W)).h_full().flex_none()),
                    )
                    .child(div().flex_1().min_w_0().h_full().flex().bg(surfaces.background).child(
                        self.gallery.clone().cached(StyleRefinement::default().flex_1().min_w_0().h_full()),
                    ))
                    .child(
                        self.editor
                            .clone()
                            .cached(StyleRefinement::default().w(px(EDITOR_W)).h_full().flex_none()),
                    ),
            )
    }
}
