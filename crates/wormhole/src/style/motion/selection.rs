//! A short selection fade shared by settings choices and navigation.

use gpui_kit::base::{Transition, transition};
use gpui_kit::*;

use super::{EASE_OUT, MOTION_FAST};

thread_local! {
    static SELECTION: Transition = Transition::new(MOTION_FAST).ease(EASE_OUT.easing());
}

#[derive(IntoElement)]
pub(in crate::style) struct SelectionFill {
    pub selected: bool,
    pub color: Hsla,
    pub radius: Pixels,
}

impl RenderOnce for SelectionFill {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let opacity = transition(
            "selection-fill",
            if self.selected { 1.0f32 } else { 0.0 },
            SELECTION.with(Clone::clone),
            window,
            cx,
        );
        div().absolute().top_0().left_0().size_full().rounded(self.radius).bg(self.color).opacity(opacity)
    }
}
