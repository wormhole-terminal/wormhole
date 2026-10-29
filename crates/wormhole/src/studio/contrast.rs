//! Contrast in the token editor: the pairs the selected token is part of,
//! and the draft's pairs that fall short, each with a fix written as an expression and Show.

use std::collections::BTreeSet;
use wormhole_text::keys::{
    STUDIO_CONTRAST, STUDIO_CONTRAST_ON_FILL, STUDIO_CONTRAST_SHOW, studio_contrast_follows,
    studio_contrast_min, studio_contrast_on, studio_low_contrast_count,
};

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::tokens::ThemeTokens;
use wormhole_core::theme::tokens::{ContrastCheck, ContrastIssue};

use super::token_editor::section;
use super::{Edit, StudioWindow};
use crate::settings::UiColors;
use crate::style::{self, Edges, TextRole};

/// The tokens whose text falls short somewhere: the list marks them and filters by them.
pub(super) fn low_contrast_tokens(tokens: &ThemeTokens) -> BTreeSet<String> {
    tokens.contrast_warnings().into_iter().map(|(fg, _, _)| fg).collect()
}

impl StudioWindow {
    /// The draft's pairs that fall short, each with its fix and Show.
    pub(super) fn low_contrast_card(
        &self,
        issues: &[ContrastIssue],
        code_font: SharedString,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut card = style::card(ui, ui.warning).flex().flex_col().gap_3().bg(ui.banner_warning).child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_label()
                .text_color(ui.warning_text)
                .child(style::icon(IconName::TriangleAlert).text_color(ui.warning))
                .child(studio_low_contrast_count(&issues.len().to_string())),
        );
        for (ix, issue) in issues.iter().enumerate() {
            let check = &issue.check;
            let fg = check.fg;
            let actions = match (&issue.fix, issue.follows) {
                (Some(fix), _) => {
                    let (token, expression) = (fix.token.clone(), fix.expression.clone());
                    Some(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new(("studio-fix", ix))
                                    .outline()
                                    .xsmall()
                                    .label(fix.label.clone())
                                    .tooltip(fix.expression.clone())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.set_value(
                                            &token,
                                            Some(expression.clone()),
                                            Edit::Field,
                                            window,
                                            cx,
                                        )
                                    })),
                            )
                            .child(
                                style::link_button(
                                    ("studio-show", ix),
                                    ui,
                                    wormhole_text::text(STUDIO_CONTRAST_SHOW),
                                    false,
                                )
                                .on_click(
                                    cx.listener(move |this, _, window, cx| this.select_token(fg, window, cx)),
                                ),
                            )
                            .into_any_element(),
                    )
                }
                (None, Some(follows)) => Some(
                    div()
                        .text_caption()
                        .text_color(ui.muted)
                        .child(studio_contrast_follows(follows))
                        .into_any_element(),
                ),
                (None, None) => None,
            };
            card = card.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .when(ix > 0, |d| d.pt_3().rule_top(ui))
                    .child(ratio_line(check, code_font.clone(), ui))
                    .child(div().text_caption().text_color(ui.faint).child(studio_contrast_on(check.bg)))
                    .children(actions),
            );
        }
        div().p_4().child(card).into_any_element()
    }
}

/// The contrast pairs a token is part of: as the fill ("Contrast on this fill") or as the
/// text, each with ✓ or ⚠, its ratio and minimum.
pub(super) fn contrast_section(
    name: &str,
    checks: &[ContrastCheck],
    code_font: SharedString,
    ui: UiColors,
) -> AnyElement {
    let fill = checks.iter().any(|c| c.bg == name);
    let title = wormhole_text::text(if fill { STUDIO_CONTRAST_ON_FILL } else { STUDIO_CONTRAST });
    section(ui)
        .gap_2()
        .child(style::section_label(ui, title))
        .children(checks.iter().map(|check| {
            let (other, on) = if check.bg == name { (check.fg, name) } else { (check.fg, check.bg) };
            let (icon, color) = if check.passes() {
                (IconName::Check, ui.success)
            } else {
                (IconName::TriangleAlert, ui.warning)
            };
            div()
                .flex()
                .items_start()
                .gap_2()
                .child(div().pt_1().child(style::icon(icon).text_color(color)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(div().truncate().text_caption().font_family(code_font.clone()).child(other))
                        .child(
                            div()
                                .truncate()
                                .text_caption()
                                .text_color(ui.faint)
                                .child(studio_contrast_on(on)),
                        ),
                )
                .child(ratio(check, ui))
        }))
        .into_any_element()
}

/// "text.link   2.6:1  min 4.5": a failing pair's first line.
fn ratio_line(check: &ContrastCheck, code_font: SharedString, ui: UiColors) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(div().flex_1().min_w_0().truncate().text_caption().font_family(code_font).child(check.fg))
        .child(ratio(check, ui))
}

/// A ratio in its status color, and the minimum it is held to.
fn ratio(check: &ContrastCheck, ui: UiColors) -> Div {
    let color = if check.passes() { ui.success_text } else { ui.warning_text };
    div()
        .flex_none()
        .flex()
        .items_baseline()
        .gap_2()
        .text_caption()
        .tabular_nums()
        .child(
            div().font_weight(FontWeight::SEMIBOLD).text_color(color).child(format!("{:.1}:1", check.ratio)),
        )
        .child(div().text_color(ui.faint).child(studio_contrast_min(&check.min.to_string())))
}
