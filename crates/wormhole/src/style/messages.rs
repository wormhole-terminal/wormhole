//! Messages: the inline banner, the readout beside a dragged resize handle
//! and the key hints of a footer.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::controls::keycap;
use super::scale::{BANNER_MIN_H, CONTROL_H, Edges, TextRole};
use crate::settings::UiColors;

/// What a [`banner`] says, which picks its color and icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerKind {
    Info,
    Warning,
    Danger,
    Success,
}

/// An inline message next to what it is about: the status color's icon and one
/// or two lines of text on that color's 10 % tint inside a 35 % outline
/// (`banner.<kind>.*`). Add a small button (`Button::new(…).small()`, ghost or outlined)
/// with `.child(…)`; it sits at the right. `flush` is the full-width, square banner of a
/// panel (the project's trust notice), drawn between hairlines of its color instead of
/// in a box.
#[track_caller]
pub fn banner(ui: UiColors, kind: BannerKind, flush: bool, text: impl Into<SharedString>) -> Div {
    let (icon_color, fill, border, icon) = match kind {
        BannerKind::Info => (ui.info, ui.banner_info, ui.banner_info_border, IconName::Info),
        BannerKind::Warning => {
            (ui.warning, ui.banner_warning, ui.banner_warning_border, IconName::TriangleAlert)
        }
        BannerKind::Danger => (ui.danger, ui.banner_danger, ui.banner_danger_border, IconName::TriangleAlert),
        BannerKind::Success => {
            (ui.success, ui.banner_success, ui.banner_success_border, IconName::ShieldCheck)
        }
    };
    let frame = div().flex().items_center().gap_3().min_h(px(BANNER_MIN_H)).py(px(BANNER_PAD_Y)).bg(fill);
    let frame = if flush {
        frame.w_full().px_4().rule_top(ui).rule_bottom(ui).border_color(border)
    } else {
        frame.pl_3().pr_2().rounded(ui.radius).outlined(ui, border)
    };
    frame
        .child(Icon::new(icon).with_size(px(super::scale::TITLE_GLYPH)).text_color(icon_color).flex_none())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_label()
                .font_weight(FontWeight::NORMAL)
                .text_color(ui.text)
                .child(text.into()),
        )
}

/// A banner's padding above and below its text: a one-line banner is [`BANNER_MIN_H`],
/// with a small button centered in it.
const BANNER_PAD_Y: f32 = 6.0;

/// A full-width notice in a panel that asks for something (the project's trust
/// callout): square, on the kind's tint between two hairlines of its color, the icon in the
/// lead column and the title (14 / 600) at x 40, then `body` (a line of explanation, the
/// commands, the one button) lined up under the title.
#[track_caller]
pub fn callout(
    ui: UiColors,
    kind: BannerKind,
    title: impl Into<SharedString>,
    body: impl IntoIterator<Item = AnyElement>,
) -> Div {
    let (icon_color, fill, border, icon) = match kind {
        BannerKind::Info => (ui.info, ui.banner_info, ui.banner_info_border, IconName::Info),
        BannerKind::Warning => (ui.warning, ui.banner_warning, ui.banner_warning_border, IconName::Shield),
        BannerKind::Danger => (ui.danger, ui.banner_danger, ui.banner_danger_border, IconName::TriangleAlert),
        BannerKind::Success => {
            (ui.success, ui.banner_success, ui.banner_success_border, IconName::ShieldCheck)
        }
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .px_4()
        .py_3()
        .bg(fill)
        .rule_top(ui)
        .rule_bottom(ui)
        .border_color(border)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(super::rows::lead(
                    Icon::new(icon).with_size(px(super::scale::GLYPH)).text_color(icon_color),
                ))
                .child(
                    div()
                        .min_w_0()
                        .text_body()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ui.text)
                        .child(title.into()),
                ),
        )
        .children(body.into_iter().map(|part| div().pl(px(CALLOUT_INDENT)).child(part)))
}

/// Where a callout's body starts: under its title, past the lead column and its gap.
const CALLOUT_INDENT: f32 = super::scale::ITEM_LEAD_W + 8.0;

/// A number shown beside the pointer while something is dragged: the width a resize
/// handle is at, and why it stopped ("430 · max"). Tooltip colors, caption digits that
/// keep their width.
#[track_caller]
pub fn readout(ui: UiColors, text: impl Into<SharedString>) -> Div {
    readout_pill(ui).px_2().py_1().child(text.into())
}

/// What a hovered command block says at the pane's bottom right: its exit code in the
/// status's color ("exit 101"), how long it took ("3.5s"), and room for the Copy output button
/// beside them (`.child(…)`). A control's height, so the button fits in it.
#[track_caller]
pub fn block_readout(ui: UiColors, code: Option<i32>, took: Option<impl Into<SharedString>>) -> Div {
    let mut out = readout_pill(ui).h(px(CONTROL_H)).flex().items_center().gap_1().pl_2().pr_1();
    if let Some(code) = code {
        let color = if code == 0 { ui.success } else { ui.danger };
        out = out.child(div().text_color(color).child(format!("exit {code}")));
    }
    if let Some(took) = took {
        let took: SharedString = took.into();
        out = out.child(if code.is_some() { format!("· {took}") } else { took.to_string() });
    }
    out
}

/// The pill a readout is written on: tooltip colors, caption digits that keep their width.
#[track_caller]
fn readout_pill(ui: UiColors) -> Div {
    div()
        .flex_none()
        .rounded(ui.radius)
        .bg(ui.tooltip)
        .text_color(ui.tooltip_text)
        .text_caption()
        .tabular_nums()
        .whitespace_nowrap()
}

/// A shortcut and what it does, in a footer or beside a control ("⏎ open", "esc close"):
/// the key cap, then the words in muted caption text.
#[track_caller]
pub fn key_hint(ui: UiColors, keys: impl Into<SharedString>, label: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap_2()
        .text_caption()
        .text_color(ui.muted)
        .child(keycap(ui, keys))
        .child(label.into())
}
