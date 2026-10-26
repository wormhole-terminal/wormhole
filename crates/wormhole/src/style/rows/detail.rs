//! A row's second line while its terminal's status changes under the Line rolls variant: the old words
//! roll up out of a clip as the new ones roll in from below, each in its own colour. Everything else a
//! change plays (the tile, the wash, the nudge) is not the line's.

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::RowTone;
use crate::style::scale::META_LINE;
use crate::style::status::StatusGlyph;
use crate::style::status_shift::StatusShift;

/// The second line rolling from the words it had to the ones the row has now.
pub struct DetailRoll {
    /// What the line said, in the tone and the font it said it in (`None`: the interface's).
    pub from: SharedString,
    pub from_tone: RowTone,
    pub from_font: Option<SharedString>,
    pub shift: StatusShift,
    /// The status the row is in now, which the shift is a change to.
    pub to: StatusGlyph,
}

/// One of the two lines of a roll, as it is drawn: its words and how they are set.
#[derive(Clone)]
pub(super) struct Words {
    pub(super) text: SharedString,
    pub(super) color: Hsla,
    pub(super) font: Option<SharedString>,
    pub(super) weight: FontWeight,
}

impl Words {
    #[track_caller]
    fn line(&self) -> Div {
        div()
            .min_w_0()
            .truncate()
            .font_weight(self.weight)
            .text_color(self.color)
            .when_some(self.font.clone(), |line, font| line.font_family(font))
            .child(self.text.clone())
    }
}

/// The line that rolls from the `old` words to `now`, `META_LINE` tall and cutting what leaves it. It takes
/// the room the line has (`flex_1`, so the row leaves no spacer after it), whatever the words are: the old
/// ones, which lie over the new and go up, are cut to that and not to the width of the new, which may be a
/// few letters. The new words stay in the layout, moved down by what is left of the roll.
#[track_caller]
pub(super) fn rolling(
    id: ElementId,
    shift: StatusShift,
    to: StatusGlyph,
    old: Words,
    now: Words,
) -> AnyElement {
    let clip = div().relative().flex_1().min_w_0().h(rems(META_LINE)).flex().items_center().overflow_hidden();
    shift.drive(clip, id, to, move |clip, pose| {
        let rolled = pose.roll;
        let arriving = now
            .line()
            .relative()
            .top(rems((1.0 - rolled) * META_LINE))
            .debug_selector(|| "detail-roll-arriving".into());
        let leaving = div()
            .absolute()
            .left_0()
            .top(rems(-rolled * META_LINE))
            .w_full()
            .h(rems(META_LINE))
            .flex()
            .items_center()
            .debug_selector(|| "detail-roll-leaving".into())
            .child(old.line());
        clip.child(if rolled < 1.0 { arriving.opacity(rolled) } else { arriving })
            .children((rolled < 1.0).then(|| leaving.opacity(1.0 - rolled)))
    })
}

#[cfg(all(test, feature = "devctl"))]
mod tests {
    use gpui_kit::prelude::*;
    use gpui_kit::{FontWeight, Hsla, TestAppContext, VisualTestContext, div, point, px, size};
    use wormhole_core::view::motion::StatusMotion;

    use super::{Words, rolling};
    use crate::style::{ShiftClock, StatusGlyph, StatusShift};

    fn words(text: &'static str) -> Words {
        Words { text: text.into(), color: Hsla::black(), font: None, weight: FontWeight::NORMAL }
    }

    /// The width of an element `rolling` draws, a quarter of the way through a roll in a line 300 wide.
    fn widths(cx: &mut TestAppContext, old: &'static str, new: &'static str) -> (f32, f32) {
        let shift = StatusShift {
            motion: StatusMotion::LineRoll,
            from: StatusGlyph::Idle,
            clock: ShiftClock::At(0.25),
        };
        let cx = cx.add_empty_window();
        cx.draw(point(px(0.0), px(0.0)), size(px(300.0), px(40.0)), |_, _| {
            div().w(px(300.0)).flex().child(rolling(
                "roll".into(),
                shift,
                StatusGlyph::Working,
                words(old),
                words(new),
            ))
        });
        let width =
            |cx: &mut VisualTestContext, name| f32::from(cx.debug_bounds(name).expect("drawn").size.width);
        (width(cx, "detail-roll-leaving"), width(cx, "detail-roll-arriving"))
    }

    #[gpui_kit::test]
    fn old_words_are_cut_to_the_line_and_not_to_the_width_of_the_new_ones(cx: &mut TestAppContext) {
        let (leaving, arriving) = widths(cx, "Editing crates/wormhole/src/style/rows/detail.rs", "Done");
        assert!(leaving > arriving * 2.0, "leaving {leaving} against arriving {arriving}");
        assert!(leaving > 250.0, "the old words have the line's room: {leaving}");
    }
}
