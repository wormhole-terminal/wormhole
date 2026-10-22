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
use wormhole_core::theme::tokens::{
    ExprError, check_expression, contrast_issues, default_rule, followers, refers_to, uses_of,
};
use wormhole_core::theme::tokens::{METRIC_TOKENS, ThemeFile, ThemeTokens, UI_TOKENS};

use super::draft::{self, is_metric, number, override_of, set_override};
use super::{Edit, StudioWindow, tokens_of, ui};
use crate::settings::{SettingsStore, UiColors, settings};
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge::hsla;
use crate::workspace::aux_window;

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
        let panel = aux_window::surfaces(cx, settings(cx).studio_translucent).panel_right;
        match self.studio.upgrade() {
            Some(studio) => studio.update(cx, |studio, cx| studio.render_token_editor(panel, cx)),
            None => div().into_any_element(),
        }
    }
}

/// A metric slider's top (px): past it, a radius is round whatever the element and a line is
/// a frame.
pub(super) const METRIC_MAX: f32 = 24.0;

/// The value forms the field takes, shown while what is typed doesn't work.
const FORMS: [(&str, &str); 5] = [
    ("#7aa2f7", "hex"),
    ("primary / 20%", "alpha"),
    ("mix(a, b, 30%)", "mix"),
    ("lighten(a, 10%)", "lighter"),
    ("readable(a, bg, 4.5)", "contrast"),
];

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

/// What is wrong with the value in the field.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct FieldError {
    /// "Unknown token “primery” at 1:1."
    pub(super) message: SharedString,
    /// A name that exists close to an unknown one, and the field's text with it in place.
    pub(super) suggestion: Option<(String, String)>,
}

impl FieldError {
    pub(super) fn plain(message: impl Into<SharedString>) -> Self {
        Self { message: message.into(), suggestion: None }
    }

    fn from_expr(err: &ExprError, src: &str) -> Self {
        let suggestion = err.suggestion.clone().zip(err.with_suggestion(src));
        Self { message: err.to_string().into(), suggestion }
    }

    /// A draft that parses but doesn't resolve: a reference cycle, mostly. The resolver
    /// names the theme and token first; the field is about one token, so that goes.
    pub(super) fn from_resolve(err: &str) -> Self {
        let what = err.rsplit_once("\": ").map_or(err, |(_, what)| what);
        let mut message = what.replace(" -> ", " \u{2192} ");
        if let Some(first) = message.get(..1) {
            message = first.to_uppercase() + &message[1..];
        }
        if !message.ends_with('.') {
            message.push('.');
        }
        Self::plain(message)
    }
}

impl StudioWindow {
    /// On every keystroke: say what is wrong with the value as it stands, without applying it.
    pub(super) fn check_field(&mut self, value: &str, cx: &mut Context<Self>) {
        let Some(name) = self.selected.clone() else { return };
        let value = value.trim();
        self.field_error = if value.is_empty() {
            None
        } else if is_metric(&name) {
            set_override(&mut ThemeFile::default(), &name, Some(value.to_string()))
                .err()
                .map(FieldError::plain)
        } else {
            check_expression(value, &name).err().map(|err| FieldError::from_expr(&err, value))
        };
        cx.notify();
    }

    /// Return: apply the value if it works (empty goes back to the rule). One that doesn't
    /// stays in the field with its error, and the draft keeps the last good value.
    pub(super) fn apply_field(&mut self, value: String, window: &mut Window, cx: &mut Context<Self>) {
        self.close_completions(cx);
        if self.field_error.is_some() && !value.is_empty() {
            return;
        }
        self.set_selected_value((!value.is_empty()).then_some(value), Edit::Field, window, cx);
    }

    /// Did you mean …? puts the name in and applies it.
    pub(super) fn take_suggestion(&mut self, fixed: String, window: &mut Window, cx: &mut Context<Self>) {
        self.expression.update(cx, |e, cx| e.set_value(fixed.clone(), window, cx));
        self.check_field(&fixed, cx);
        self.apply_field(fixed, window, cx);
    }

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
                    .child("Pick a token in the list, or click a color in the gallery, to edit it here.")
                    .into_any_element(),
            ],
        };
        let issues = contrast_issues(&tokens, &self.draft.ui);
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
            let s = if follow_count == 1 { "" } else { "s" };
            description = format!("{description}. {follow_count} token{s} follow it.");
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
                    .child(div().flex_none().text_caption().text_color(ui.faint).child(group)),
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
        let overridden = override_of(&self.draft, name).is_some();
        let rule = default_rule(name, tokens.appearance);
        let (tag, tag_color) = if self.field_error.is_some() {
            ("last good value", ui.warning_text)
        } else if overridden {
            ("override", ui.primary)
        } else if rule.is_some() {
            ("derived", ui.muted)
        } else {
            ("theme", ui.muted)
        };
        let reset =
            Button::new("token-reset").ghost().xsmall().icon(IconName::RotateCcw).label("Reset").on_click(
                cx.listener(|this, _, window, cx| this.set_selected_value(None, Edit::Field, window, cx)),
            );
        let head = div()
            .flex()
            .items_center()
            .child(div().flex_1().child(style::section_label(ui, "Value")))
            .when(overridden, |d| d.child(reset));

        let mut body = section(ui).gap_2().child(head);
        if metric {
            body = body.child(self.metric_editor(name, tokens, rule, ui, cx));
        } else {
            let color = tokens.get(name).unwrap_or(crate::theme_bridge::MISSING_TOKEN);
            let alpha = if color.a >= 1.0 {
                "sRGB · opaque".to_string()
            } else {
                format!("sRGB · {:.0}% alpha", color.a * 100.0)
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
                                        override_of(&self.draft, name),
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

    /// The error under the field, Did you mean …?, and the forms a value can take.
    fn field_problem(
        &self,
        err: FieldError,
        code_font: SharedString,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let forms = FORMS.iter().map(|(form, what)| {
            div()
                .flex()
                .items_center()
                .child(div().flex_1().font_family(code_font.clone()).text_color(ui.text).child(*form))
                .child(div().text_color(ui.faint).child(*what))
        });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .text_caption()
                    .text_color(ui.danger_text)
                    .child(style::icon(IconName::TriangleAlert).text_color(ui.danger).flex_none())
                    .child(div().flex_1().min_w_0().child(err.message.clone())),
            )
            .children(err.suggestion.map(|(name, fixed)| {
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .pl_6()
                    .text_caption()
                    .text_color(ui.muted)
                    .child("Did you mean")
                    .child(
                        style::link_button("studio-did-you-mean", ui, name, false)
                            .font_family(code_font.clone())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.take_suggestion(fixed.clone(), window, cx)
                            })),
                    )
                    .child("?")
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_2()
                    .rounded(ui.radius)
                    .bg(ui.hover)
                    .text_caption()
                    .child(div().text_color(ui.muted).child("Forms"))
                    .children(forms),
            )
            .into_any_element()
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
        let following = override_of(&self.draft, name).is_none();
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
                .child(format!("Follow {source} ({} px)", number(source_value)))
        });
        let help = if name.ends_with("width") {
            "A number of pixels. No color picker for metrics."
        } else {
            "A number of pixels; a radius above half the height is fully round. No color picker for metrics."
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
                        format!("{} px", number(value)),
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
                            .child(style::section_label(ui, &format!("Preview at {}", number(value)))),
                    )
                    .child(div().text_caption().text_color(ui.faint).child(format!("{uses} uses"))),
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
        let overridden = override_of(&self.draft, name).is_some();
        let explanation = match (&rule, overridden) {
            (Some(_), true) => {
                "This theme sets it, so the rule isn't used; Reset goes back to the rule.".to_string()
            }
            (Some(rule), false) => match refers_to(rule).as_slice() {
                [] => "A fixed value. Typing a value overrides it; Reset goes back to the rule.".to_string(),
                names => format!(
                    "Follows {}. Typing a value overrides it; Reset goes back to the rule.",
                    and_list(names)
                ),
            },
            (None, _) => format!("From the palette of {}. Typing a value overrides it.", tokens.name),
        };
        section(ui)
            .gap_1()
            .child(style::section_label(ui, "Rule"))
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
                    .child(div().flex_1().child(style::section_label(ui, "Used by")))
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
                "No component draws with it directly; what follows it does."
            } else {
                "Outlined in the gallery while selected."
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
        return format!("Terminal palette color {slot}");
    }
    if let Some(key) = name.strip_prefix("syntax.") {
        return format!("Code highlighting: {key}");
    }
    match name {
        "terminal.background" => "The terminal's background".into(),
        "terminal.foreground" => "The terminal's text".into(),
        "terminal.cursor" => "The terminal's cursor".into(),
        "terminal.selection_background" => "Selected text in the terminal".into(),
        _ => String::new(),
    }
}

/// "a", "a and b", "a, b and c".
fn and_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::{FieldError, and_list, value_line};

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
