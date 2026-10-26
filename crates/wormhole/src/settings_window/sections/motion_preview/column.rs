//! The sidebar hiding and showing: a miniature window with the sidebar's rows and a terminal beside
//! them, drawn from the pose of the chosen `ColumnMotion` (core's `pose::column`), the same one the window
//! draws. The column slides over the terminal and the terminal moves with its edge, as the window's
//! sidebar carries it; the terminal's own size never changes while it moves (it changes once, when the
//! column has landed), so the preview moves it and never squeezes it. Classic slides on timed curves,
//! Soft spring on the soft spring, and Content after fades the rows out before the column goes and in after
//! it has begun to come, over a surface that stays.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::pose::column::{self, ColumnPose};
use wormhole_text::keys::SETTINGS_MOTION_COLUMN_TERMINAL;
use wormhole_text::text;

use super::{ASKING, Spot, WORKING, metrics, row, since};
use crate::settings::{UiColors, settings};
use crate::style::{Edges, TextRole, preview_stage};

/// The miniature sidebar's width.
const SIDEBAR_W: f32 = 200.0;

/// When the column hides and when it shows again, as shares of the loop: the slower of the three
/// variants is over well before the next begins.
const HIDES: f32 = 0.15;
const SHOWS: f32 = 0.55;

/// The column at `t` of the loop: in until it hides, going until it is out, out until it shows, coming
/// until it is in. Before it hides it is the picture before anything moves, which Reduce Motion holds.
fn at(variant: wormhole_core::view::motion::ColumnMotion, t: f32) -> ColumnPose {
    if t < SHOWS {
        column::pose(variant, false, 1.0, since(t, HIDES))
    } else {
        column::pose(variant, true, 0.0, since(t, SHOWS))
    }
}

pub(super) fn stage(ui: UiColors, t: f32, cx: &App) -> Div {
    let row_h = metrics().row_h;
    let height = 3.0 * row_h;
    let ColumnPose { shown, content } = at(settings(cx).motion.column, t);
    // The terminal rides the column's edge, whole: it is moved, not sized.
    let terminal = Spot::row(0.0, height).moved(SIDEBAR_W * shown, 0.0).put(|| {
        div()
            .size_full()
            .p_3()
            .bg(ui.background)
            .child(div().text_caption().text_color(ui.muted).child(text(SETTINGS_MOTION_COLUMN_TERMINAL)))
            .into_any_element()
    });
    let sidebar = Spot::wide(SIDEBAR_W, height).moved(-SIDEBAR_W * (1.0 - shown), 0.0).put(|| {
        div()
            .size_full()
            .bg(ui.panel)
            .rule_right(ui)
            // What is in the column fades; the surface under it stays.
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .when(content < 1.0, |rows| rows.opacity(content))
                    .child(div().h(px(row_h)).child(row(&WORKING, "motion-column-0", ui, cx)))
                    .child(div().h(px(row_h)).child(row(&ASKING, "motion-column-1", ui, cx))),
            )
            .into_any_element()
    });
    preview_stage(ui, height).children(terminal).children(sidebar)
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::motion::ColumnMotion;

    use super::{HIDES, SHOWS, at};
    use crate::settings_window::sections::motion_preview::LOOP;

    #[test]
    fn the_loop_starts_with_the_column_in_and_ends_with_it_in() {
        for &variant in <ColumnMotion as wormhole_core::view::motion::MotionChoice>::ALL {
            let (first, last) = (at(variant, 0.0), at(variant, 0.999));
            assert_eq!(
                (first.shown, first.content),
                (1.0, 1.0),
                "{variant:?}: the picture before anything moves"
            );
            assert_eq!((last.shown, last.content), (1.0, 1.0), "{variant:?}: it is back and at rest");
        }
    }

    #[test]
    fn every_variant_goes_out_and_comes_back_inside_the_loop() {
        for &variant in <ColumnMotion as wormhole_core::view::motion::MotionChoice>::ALL {
            let out = at(variant, SHOWS - 0.01);
            assert_eq!(out.shown, 0.0, "{variant:?} is out before it shows again");
            assert!(at(variant, HIDES + 0.02).shown < 1.0 || variant == ColumnMotion::ContentAfter);
        }
        assert!(LOOP.as_secs_f32() * (SHOWS - HIDES) > 1.0, "room to rest between the two");
    }
}
