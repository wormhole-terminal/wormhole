//! The token editor's value field: what is wrong with what is typed (checked on every keystroke),
//! applying it on Return, the "Did you mean" suggestion, and the forms a value can take.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::tokens::{ExprError, ThemeFile, check_expression};

use super::document::{is_metric, set_override};
use super::{Edit, StudioWindow};
use crate::settings::UiColors;
use crate::style::{self, TextRole};

/// The value forms the field takes, shown while what is typed doesn't work.
const FORMS: [(&str, &str); 5] = [
    ("#7aa2f7", "hex"),
    ("primary / 20%", "alpha"),           // text: code sample
    ("mix(a, b, 30%)", "mix"),            // text: code sample
    ("lighten(a, 10%)", "lighter"),       // text: code sample
    ("readable(a, bg, 4.5)", "contrast"), // text: code sample,
];

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

    /// The error under the field, Did you mean …?, and the forms a value can take.
    pub(super) fn field_problem(
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
                    .child(wormhole_text::text(wormhole_text::keys::STUDIO_DID_YOU_MEAN))
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
                    .child(
                        div()
                            .text_color(ui.muted)
                            .child(wormhole_text::text(wormhole_text::keys::STUDIO_FORMS)),
                    )
                    .children(forms),
            )
            .into_any_element()
    }
}
