//! The row that opens more: a label, what is behind it ("17"), and a chevron that turns down while the
//! rest is shown under the row. It is a [`super::setting`]'s height, with its hairline above, so it sits
//! among the rows of a list.

use gpui_kit::*;

use crate::settings::UiColors;
use crate::ui::{Look, MoreRowProps};

/// A row that opens more, drawn by the `MoreRow` component. `value` says what is behind it (a count); the
/// caller adds `.on_click(…)` that flips what `open` reads, and draws what is shown under the row. A tab
/// stop, so ↩ and Space open it as a click does; VoiceOver reads a button that is expanded or not.
#[track_caller]
pub fn more_row(
    id: impl Into<ElementId>,
    _: UiColors,
    label: impl Into<SharedString>,
    value: Option<SharedString>,
    open: bool,
    cx: &App,
) -> Stateful<Div> {
    MoreRowProps { id: id.into(), label: label.into(), value, open, closed: !open, on_click: None }
        .render(&Look::of(cx))
}
