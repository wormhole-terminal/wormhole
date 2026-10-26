//! The sidebar hiding and showing: a miniature window with the sidebar's rows and a terminal beside
//! them. Classic: the column slides on a timed curve, in over the terminal at its full width and out
//! over it, and the terminal changes its width once, when the column is in or the moment it starts
//! to go: never a little at a time.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_text::keys::SETTINGS_MOTION_COLUMN_TERMINAL;
use wormhole_text::text;

use super::{ASKING, Spot, WORKING, metrics, row, span};
use crate::settings::UiColors;
use crate::style::motion::{EASE_IN, EASE_OUT, MOTION_PANEL, MOTION_PANEL_EXIT};
use crate::style::{Edges, TextRole, preview_stage};

/// The miniature sidebar's width.
const SIDEBAR_W: f32 = 200.0;

pub(super) fn stage(ui: UiColors, t: f32, cx: &App) -> Div {
    let row_h = metrics().row_h;
    let height = 3.0 * row_h;
    let (hides, shows) = (span(0.15, MOTION_PANEL_EXIT), span(0.55, MOTION_PANEL));
    // The terminal takes the column's room while the column is in layout, and not while it slides.
    let room = if t < hides.start || t >= shows.end { SIDEBAR_W } else { 0.0 };
    let terminal = Spot::row(0.0, height).moved(room, 0.0).put(|| {
        div()
            .size_full()
            .p_3()
            .bg(ui.background)
            .child(div().text_caption().text_color(ui.muted).child(text(SETTINGS_MOTION_COLUMN_TERMINAL)))
            .into_any_element()
    });
    // Out on the way an exit goes (accelerating), back in on the way an entrance does.
    let shown = if t < shows.start {
        1.0 - EASE_IN.easing()(hides.progress(t))
    } else {
        EASE_OUT.easing()(shows.progress(t))
    };
    let sidebar = Spot::wide(SIDEBAR_W, height).moved(-SIDEBAR_W * (1.0 - shown), 0.0).put(|| {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(ui.panel)
            .rule_right(ui)
            .child(div().h(px(row_h)).child(row(&WORKING, "motion-column-0", ui, cx)))
            .child(div().h(px(row_h)).child(row(&ASKING, "motion-column-1", ui, cx)))
            .into_any_element()
    });
    preview_stage(ui, height).children(terminal).children(sidebar)
}
