//! `SettingsPage`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::ui::{Look};

/// What `SettingsPage` is drawn from.
pub struct SettingsPageProps {
    pub content: AnyElement,
}

impl SettingsPageProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, _look: &Look) -> Div {
        let Self { content } = self;

        div()
            .flex()
            .flex_col()
            .w_full()
            .pt(px(44.0)).pb(px(160.0)).px(px(32.0))
            .child(div()
                .flex()
                .flex_col()
                .gap(px(22.0))
                .w_full()
                .max_w(px(536.0))
                .child(content))
    }
}
