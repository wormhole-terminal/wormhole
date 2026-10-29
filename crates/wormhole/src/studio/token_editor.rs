//! The token editor, 320 wide on the right: the selected token's name,
//! group and purpose; its value (the picker, or a metric's stepper and slider) with the one
//! field for a hex value or an expression, checked as it is typed; the rule it derives from;
//! what uses it; the contrast pairs it is part of; and the draft's low-contrast pairs, each
//! with a fix written as an expression.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::color_picker::ColorPicker;
use gpui_kit::component::input::Input;
use gpui_kit::component::slider::Slider;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::tokens::{METRIC_TOKENS, ThemeTokens, UI_TOKENS};
use wormhole_core::theme::tokens::{contrast_issues, default_rule, followers, refers_to, uses_of};

use super::document::{is_metric, number};
use super::draft;
use super::origin_tag::reach_chip;
use super::{Edit, StudioWindow, tokens_of, ui};
use crate::settings::{SettingsStore, UiColors, settings};
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge::hsla;
use wormhole_text::keys::{
    STUDIO_DESCRIBE_BACKGROUND, STUDIO_DESCRIBE_CURSOR, STUDIO_DESCRIBE_FOREGROUND,
    STUDIO_DESCRIBE_SELECTION, STUDIO_METRIC_HELP_RADIUS, STUDIO_METRIC_HELP_WIDTH, STUDIO_PICK_A_TOKEN,
    STUDIO_RESET, STUDIO_RESET_BASE, STUDIO_RULE_FIXED, STUDIO_RULE_SET_HERE, STUDIO_SECTION_RULE,
    STUDIO_SECTION_USED_BY, STUDIO_SECTION_VALUE, STUDIO_SRGB_OPAQUE, STUDIO_TAG_LAST_GOOD,
    STUDIO_USED_BY_NONE, STUDIO_USED_BY_OUTLINED, studio_and_list, studio_describe_ansi,
    studio_describe_syntax, studio_followers, studio_metric_follow, studio_metric_preview, studio_metric_px,
    studio_metric_uses, studio_rule_follows, studio_rule_palette, studio_srgb_alpha,
};
use wormhole_text::text;

/// The editor's width.
pub(super) const EDITOR_W: f32 = 320.0;

/// The token editor as a view of its own: scrolling it, or the gallery or the list, redraws that
/// one alone. Its content is the window's (the draft, the field, the contrast cards) and is made
/// by the window's own methods, whose handlers answer to the window.
pub(super) struct TokenEditorView {
    studio: WeakEntity<StudioWindow>,
    /// How often the view was drawn (the counting tests).
    #[cfg(test)]
    pub(super) renders: usize,
    _subscriptions: Vec<Subscription>,
}

impl TokenEditorView {
    pub(super) fn new(studio: &Entity<StudioWindow>, cx: &mut Context<Self>) -> Self {
        Self {
            studio: studio.downgrade(),
            #[cfg(test)]
            renders: 0,
            // The window says when what the editor shows changes.
            _subscriptions: vec![
                cx.observe(studio, |_, _, cx| cx.notify()),
                cx.observe_global::<SettingsStore>(|_, cx| cx.notify()),
            ],
        }
    }
}

impl Render for TokenEditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        {
            self.renders += 1;
        }
        let panel = style::window_surfaces(cx, settings(cx).studio_translucent).panel_right;
        match self.studio.upgrade() {
            Some(studio) => studio.update(cx, |studio, cx| studio.render_token_editor(panel, cx)),
            None => div().into_any_element(),
        }
    }
}

/// A metric slider's top (px): past it, a radius is round whatever the element and a line is
/// a frame.
pub(super) const METRIC_MAX: f32 = 24.0;
/// The line next to the swatch: what the draft sets itself, as typed, otherwise the color it
/// resolves to. While the field holds a value that doesn't work the draft keeps the last good one,
/// and the line says that as the recipe it comes from (`primary / 20%`) rather than as a code,
/// so the working version stands above the broken text.
fn value_line(own: Option<String>, hex: String, rule: Option<&str>, failing: bool) -> String {
    own.unwrap_or_else(|| match rule {
        Some(rule) if failing => rule.to_string(),
        _ => hex,
    })
}

impl StudioWindow {
    fn render_token_editor(&self, panel: Hsla, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let tokens = tokens_of(cx);
        let (code_font, _) = crate::fonts::code_font(cx);
        let sections = match self.selected.clone() {
            Some(name) => self.token_sections(&name, &tokens, code_font.clone(), ui, cx),
            None => vec![
                div()
                    .px_4()
                    .py_4()
                    .text_body()
                    .text_color(ui.muted)
                    .child(text(STUDIO_PICK_A_TOKEN))
                    .into_any_element(),
            ],
        };
        let issues = contrast_issues(&tokens, &self.doc.file.ui);
        div()
            .id("studio-token-editor")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .rule_left(ui)
            .bg(panel)
            .children(sections)
            .when(!issues.is_empty(), |d| d.child(self.low_contrast_card(&issues, code_font, ui, cx)))
            .into_any_element()
    }

    fn token_sections(
        &self,
        name: &str,
        tokens: &ThemeTokens,
        code_font: SharedString,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let info = UI_TOKENS.iter().chain(METRIC_TOKENS).find(|t| t.name == name);
        let group = draft::group_of(name).unwrap_or_default();
        let follow_count = followers(name, tokens.appearance).len();
        let mut description = info.map(|t| t.description.to_string()).unwrap_or_else(|| describe(name));
        if follow_count > 0 {
            description = studio_followers(i64::try_from(follow_count).unwrap_or(i64::MAX), &description);
        }
        let header = section(ui)
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_body()
                            .font_weight(FontWeight::SEMIBOLD)
                            .font_family(code_font.clone())
                            .child(SharedString::from(name.to_string())),
                    )
                    .child(div().flex_none().text_caption().text_color(ui.faint).child(group))
                    .children(self.doc.is_pair().then(|| {
                        let this = cx.weak_entity();
                        let shown = self.doc.shown.clone().unwrap_or_default();
                        let reach =
                            super::document::places::reach(&self.doc.file, self.doc.shown.as_deref(), name);
                        reach_chip(name, reach, &shown, ui.faint, ui, move |token, window, cx| {
                            this.update(cx, |studio, cx| studio.toggle_reach(&token, window, cx)).ok();
                        })
                    })),
            )
            .child(div().text_caption().text_color(ui.muted).child(description));
        let mut sections =
            vec![header.into_any_element(), self.value_section(name, tokens, code_font.clone(), ui, cx)];
        sections.push(self.rule_section(name, tokens, code_font.clone(), ui));
        sections.push(self.used_by_section(name, ui));
        let checks = tokens.contrast_checks_of(name);
        if !checks.is_empty() {
            sections.push(super::contrast::contrast_section(name, &checks, code_font, ui));
        }
        sections
    }

    fn value_section(
        &self,
        name: &str,
        tokens: &ThemeTokens,
        code_font: SharedString,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let metric = is_metric(name);
        let overridden = self.doc.own(name).is_some();
        let rule = default_rule(name, tokens.appearance);
        let (tag, tag_color) = if self.field_error.is_some() {
            (text(STUDIO_TAG_LAST_GOOD), ui.warning_text)
        } else if overridden {
            ("override", ui.primary)
        } else if rule.is_some() {
            ("derived", ui.muted)
        } else {
            ("theme", ui.muted)
        };
        // Reset drops the theme's own value: back to what it was copied as, else what is worked out.
        let has_base =
            super::document::places::base_value(&self.doc.file, self.doc.shown.as_deref(), name).is_some();
        let reset = Button::new("token-reset")
            .ghost()
            .xsmall()
            .icon(IconName::RotateCcw)
            .label(text(if has_base { STUDIO_RESET_BASE } else { STUDIO_RESET }))
            .on_click(
                cx.listener(|this, _, window, cx| this.set_selected_value(None, Edit::Field, window, cx)),
            );
        let head = div()
            .flex()
            .items_center()
            .child(div().flex_1().child(style::section_label(ui, text(STUDIO_SECTION_VALUE))))
            .when(overridden, |d| d.child(reset));

        let mut body = section(ui).gap_2().child(head);
        if metric {
            body = body.child(self.metric_editor(name, tokens, rule, ui, cx));
        } else {
            let color = tokens.get(name).unwrap_or(crate::theme_bridge::MISSING_TOKEN);
            let alpha = if color.a >= 1.0 {
                text(STUDIO_SRGB_OPAQUE).to_string()
            } else {
                studio_srgb_alpha(&format!("{:.0}", color.a * 100.0))
            };
            let palette: Vec<Hsla> = tokens.terminal.ansi.iter().map(|c| hsla(*c)).collect();
            body = body
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(ColorPicker::new(&self.picker).featured_colors(palette))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(div().truncate().text_body().font_family(code_font.clone()).child(
                                    value_line(
                                        self.doc.own(name),
                                        color.to_hex(),
                                        rule.as_deref(),
                                        self.field_error.is_some(),
                                    ),
                                ))
                                .child(div().text_caption().text_color(ui.faint).child(alpha)),
                        )
                        .child(style::status_badge(ui, tag_color, tag)),
                )
                .child(
                    // The wrapper takes the list's keys ahead of the window (`completion`).
                    self.completion_keys(div(), cx).child(
                        Input::new(&self.expression)
                            .small()
                            .font_family(code_font.clone())
                            .when(self.field_error.is_some(), |i| {
                                i.focus_bordered(false).border_color(ui.danger)
                            }),
                    ),
                )
                .children(self.render_completions(code_font.clone(), ui, cx))
                // Mid-name, the list is the help: the error waits until it is gone.
                .children(
                    self.field_error
                        .clone()
                        .filter(|_| self.completion.is_none())
                        .map(|err| self.field_problem(err, code_font, ui, cx)),
                );
        }
        body.into_any_element()
    }

    /// A metric: a stepper and a slider instead of the picker, Follow for one whose
    /// default is another metric's, and a preview of what it shapes.
    fn metric_editor(
        &self,
        name: &str,
        tokens: &ThemeTokens,
        rule: Option<String>,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let value = tokens.metrics.get(name).unwrap_or_default();
        let step = |delta: f32| {
            let this = cx.weak_entity();
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                let next = (value + delta).clamp(0.0, METRIC_MAX);
                this.update(cx, |studio, cx| {
                    studio.set_selected_value(Some(number(next)), Edit::Field, window, cx)
                })
                .ok();
            }
        };
        let following = self.doc.own(name).is_none();
        let follow = rule.map(|source| {
            let source_value = tokens.metrics.get(&format!("metrics.{source}")).unwrap_or_default();
            let this = cx.weak_entity();
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_caption()
                .text_color(ui.muted)
                .child(style::toggle("studio-follow", following, move |on, window, cx| {
                    this.update(cx, |studio, cx| {
                        let value = (!on).then(|| number(value));
                        studio.set_selected_value(value, Edit::Field, window, cx);
                    })
                    .ok();
                }))
                .child(studio_metric_follow(&source.to_string(), &number(source_value)))
        });
        let help = if name.ends_with("width") {
            text(STUDIO_METRIC_HELP_WIDTH)
        } else {
            text(STUDIO_METRIC_HELP_RADIUS)
        };
        let uses = uses_of(name).len();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(style::stepper(
                        ui,
                        "studio-metric",
                        studio_metric_px(&number(value)),
                        step(-1.0),
                        step(1.0),
                    ))
                    .child(div().flex_1().child(Slider::new(&self.slider))),
            )
            .children(follow)
            .child(div().text_caption().text_color(ui.faint).child(help))
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .child(style::section_label(ui, &studio_metric_preview(&number(value)))),
                    )
                    .child(
                        div()
                            .text_caption()
                            .text_color(ui.faint)
                            .child(studio_metric_uses(i64::try_from(uses).unwrap_or(i64::MAX))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .rounded(ui.radius)
                    .outlined(ui, ui.border)
                    .child(style::count_badge(ui, "3"))
                    .child(style::keycap(ui, "⌘1"))
                    .child(style::branch_chip(ui, Some("ui-unify"), None))
                    .child(style::status_badge(ui, ui.success_text, "trusted")),
            )
            .into_any_element()
    }

    fn rule_section(
        &self,
        name: &str,
        tokens: &ThemeTokens,
        code_font: SharedString,
        ui: UiColors,
    ) -> AnyElement {
        let rule = default_rule(name, tokens.appearance);
        let overridden = self.doc.own(name).is_some();
        let explanation = match (&rule, overridden) {
            (Some(_), true) => text(STUDIO_RULE_SET_HERE).to_string(),
            (Some(rule), false) => match refers_to(rule).as_slice() {
                [] => text(STUDIO_RULE_FIXED).to_string(),
                names => studio_rule_follows(&and_list(names)),
            },
            (None, _) => studio_rule_palette(&tokens.name),
        };
        section(ui)
            .gap_1()
            .child(style::section_label(ui, text(STUDIO_SECTION_RULE)))
            .children(
                rule.map(|rule| div().text_caption().font_family(code_font).text_color(ui.text).child(rule)),
            )
            .child(div().text_caption().text_color(ui.faint).child(explanation))
            .into_any_element()
    }

    fn used_by_section(&self, name: &str, ui: UiColors) -> AnyElement {
        let uses = uses_of(name);
        section(ui)
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(div().flex_1().child(style::section_label(ui, text(STUDIO_SECTION_USED_BY))))
                    .child(div().text_caption().text_color(ui.faint).child(uses.len().to_string())),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    // Grey, so the row of names does not read as an override.
                    .children(uses.iter().map(|u| style::neutral_chip(ui, u.label))),
            )
            .child(div().text_caption().text_color(ui.faint).child(if uses.is_empty() {
                text(STUDIO_USED_BY_NONE)
            } else {
                text(STUDIO_USED_BY_OUTLINED)
            }))
            .into_any_element()
    }
}

/// One block of the editor: inset 16, a hairline under it.
#[track_caller]
pub(super) fn section(ui: UiColors) -> Div {
    div().flex().flex_col().px_4().py_3().rule_bottom(ui)
}

/// What a token without a catalogue entry is: a terminal color, a palette slot, a syntax color.
fn describe(name: &str) -> String {
    if let Some(slot) = name.strip_prefix("terminal.ansi.") {
        return studio_describe_ansi(slot);
    }
    if let Some(key) = name.strip_prefix("syntax.") {
        return studio_describe_syntax(key);
    }
    match name {
        "terminal.background" => text(STUDIO_DESCRIBE_BACKGROUND).into(),
        "terminal.foreground" => text(STUDIO_DESCRIBE_FOREGROUND).into(),
        "terminal.cursor" => text(STUDIO_DESCRIBE_CURSOR).into(),
        "terminal.selection_background" => text(STUDIO_DESCRIBE_SELECTION).into(),
        _ => String::new(),
    }
}

/// "a", "a and b", "a, b and c".
fn and_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => studio_and_list(&rest.join(", "), last),
    }
}

#[cfg(test)]
mod tests {
    use super::{and_list, value_line};
    use crate::studio::token_field::FieldError;

    /// After a typo the line above the field is the expression that still stands.
    #[test]
    fn the_value_line_says_the_expression_after_a_mistake() {
        let hex = || "#7aa2f733".to_string();
        // Fine: the draft's own text, else the code the token resolves to.
        assert_eq!(value_line(None, hex(), Some("primary / 20%"), false), "#7aa2f733");
        assert_eq!(
            value_line(Some("primary / 30%".into()), hex(), Some("primary / 20%"), false),
            "primary / 30%"
        );
        // A mistake in the field: what the token follows, or what the draft set, as typed.
        assert_eq!(value_line(None, hex(), Some("primary / 20%"), true), "primary / 20%");
        assert_eq!(value_line(Some("#112233".into()), hex(), Some("primary / 20%"), true), "#112233");
        // A color with no rule (a palette color) has no recipe to show: its code.
        assert_eq!(value_line(None, hex(), None, true), "#7aa2f733");
    }

    #[test]
    fn a_cycle_reads_as_a_path() {
        let err = FieldError::from_resolve(
            "theme \"Tokyo Night Custom\": ui \"primary\": reference cycle: primary -> border.focus -> primary",
        );
        assert_eq!(err.message.as_ref(), "Reference cycle: primary \u{2192} border.focus \u{2192} primary.");
        assert_eq!(err.suggestion, None);
    }

    #[test]
    fn names_are_listed_in_words() {
        let names = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(and_list(&names(&["primary"])), "primary");
        assert_eq!(and_list(&names(&["sidebar.background", "primary"])), "sidebar.background and primary");
        assert_eq!(and_list(&names(&["a", "b", "c"])), "a, b and c");
    }
}
