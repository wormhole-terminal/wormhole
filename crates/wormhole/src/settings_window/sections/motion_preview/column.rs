//! The sidebar hiding and showing: a miniature window with the sidebar's rows and a terminal beside
//! them. Today the column slides on a timed curve, in over the terminal at its full width and out
//! over it, and the terminal changes its width once, when the column is in or the moment it starts
//! to go: never a little at a time.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_text::keys::{SETTINGS_MOTION_COLUMN_PREVIEW_LABEL, SETTINGS_MOTION_COLUMN_TERMINAL};
use wormhole_text::text;

use super::{ASKING, Pose, Scene, WORKING, metrics, show, span};
use crate::settings::settings;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::motion::{EASE_IN, EASE_OUT, MOTION_PANEL, MOTION_PANEL_EXIT};
use crate::style::{Edges, TextRole, preview_stage};

/// The miniature sidebar's width.
const SIDEBAR_W: f32 = 200.0;

pub(in crate::settings_window::sections) fn column(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> Option<AnyElement> {
    let ui = ui(cx);
    let scene = Scene::of("motion-column", settings(cx).motion.column);
    let row_h = metrics().row_h;
    let (hides, shows) = (span(0.15, MOTION_PANEL_EXIT), span(0.55, MOTION_PANEL));
    // The terminal takes the column's room while the column is in layout: not while it slides.
    let terminal = div()
        .size_full()
        .p_3()
        .bg(ui.background)
        .child(div().text_caption().text_color(ui.muted).child(text(SETTINGS_MOTION_COLUMN_TERMINAL)));
    let sidebar = div()
        .size_full()
        .flex()
        .flex_col()
        .bg(ui.panel)
        .rule_right(ui)
        .child(div().h(px(row_h)).child(WORKING.row("motion-column", 0, ui, cx)))
        .child(div().h(px(row_h)).child(ASKING.row("motion-column", 1, ui, cx)))
        .into_any_element();
    let stage = preview_stage(ui, 3.0 * row_h)
        .child(scene.place(0, None, 0.0, 3.0 * row_h, terminal.into_any_element(), move |t| Pose {
            dx: if t < hides.start || t >= shows.end { SIDEBAR_W } else { 0.0 },
            ..Pose::REST
        }))
        .child(scene.place(1, Some(SIDEBAR_W), 0.0, 3.0 * row_h, sidebar, move |t| {
            // In, then out on the way an exit takes (accelerating), back in on the way an entrance does.
            let shown = if t < shows.start {
                1.0 - EASE_IN.easing()(hides.progress(t))
            } else {
                EASE_OUT.easing()(shows.progress(t))
            };
            Pose { dx: -SIDEBAR_W * (1.0 - shown), ..Pose::REST }
        }));
    Some(show(ui, "motion-column-preview", text(SETTINGS_MOTION_COLUMN_PREVIEW_LABEL), stage))
}
