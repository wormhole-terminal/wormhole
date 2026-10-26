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

/// The line that rolls from the `old` words to `now`, `META_LINE` tall and cutting what leaves it. The new
/// words stay in the layout (so the line is as wide as they are), moved down by what is left of the
/// roll; the old ones lie over it and go up.
#[track_caller]
pub(super) fn rolling(
    id: ElementId,
    shift: StatusShift,
    to: StatusGlyph,
    old: Words,
    now: Words,
) -> AnyElement {
    let clip = div().relative().min_w_0().h(rems(META_LINE)).flex().items_center().overflow_hidden();
    shift.drive(clip, id, to, move |clip, pose| {
        let rolled = pose.roll;
        let arriving = now.line().relative().top(rems((1.0 - rolled) * META_LINE));
        let leaving = div()
            .absolute()
            .left_0()
            .top(rems(-rolled * META_LINE))
            .max_w_full()
            .h(rems(META_LINE))
            .flex()
            .items_center()
            .child(old.line());
        clip.child(if rolled < 1.0 { arriving.opacity(rolled) } else { arriving })
            .children((rolled < 1.0).then(|| leaving.opacity(1.0 - rolled)))
    })
}
