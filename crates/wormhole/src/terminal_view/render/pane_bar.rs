//! The bar along the bottom of a pane: a program asks to read the clipboard (three buttons), or a
//! notice that goes by itself (a copy, a refusal).

use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::clipboard::Choice;

use crate::settings::store_in;
use crate::style::{self, BannerKind};
use crate::terminal_view::TerminalView;
use crate::terminal_view::pane_bar::Bar;

/// A grid at least this wide has room for the long label of the second button.
const ROOMY_PX: f32 = 560.0;

impl TerminalView {
    /// The bar over the grid's bottom edge (above the ended shell's bar, if that is shown);
    /// nothing while there is nothing to say.
    pub(super) fn render_pane_bar(&self, exited: bool, cx: &mut Context<Self>) -> Option<AnyElement> {
        let bar = self.pane_bar()?;
        let ui = store_in(&self.profile, cx).ui;
        let layout = self.layout.get();
        let roomy = layout.cols as f32 * f32::from(layout.cell.width) >= ROOMY_PX;
        let frame = match bar {
            Bar::Ask(program) => style::banner(
                ui,
                BannerKind::Warning,
                false,
                format!("{program} wants to read the clipboard"),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_1()
                    .child(choice_button(
                        "clipboard-once",
                        "Allow once",
                        "Give it the clipboard's text this time",
                        Choice::Once,
                        cx,
                    ))
                    .child(choice_button(
                        "clipboard-always",
                        if roomy { "Always for this terminal" } else { "Always" },
                        "Give it the clipboard whenever it asks, until this terminal closes",
                        Choice::Always,
                        cx,
                    ))
                    .child(choice_button("clipboard-deny", "Deny", "Give it nothing", Choice::Deny, cx)),
            ),
            Bar::Notice(kind, text) => style::banner(ui, kind, false, text.to_string()),
        };
        let right = px(Self::right_inset(&self.profile, cx));
        Some(
            frame
                .id("pane-bar")
                .absolute()
                .left(px(crate::terminal_view::PADDING_X))
                .right(right)
                .when(exited, |this| this.bottom(px(crate::style::TITLE_ROW_H)))
                .when(!exited, |this| this.bottom_0())
                .mb_2()
                .cursor(CursorStyle::Arrow)
                // The pane below it would start a selection.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .into_any_element(),
        )
    }
}

fn choice_button(
    id: &'static str,
    label: &'static str,
    tooltip: &'static str,
    choice: Choice,
    cx: &mut Context<TerminalView>,
) -> Button {
    Button::new(id)
        .small()
        .outline()
        .label(label)
        .tooltip(tooltip)
        .on_click(cx.listener(move |this, _, _, cx| this.answer_clipboard_ask(choice, cx)))
}
