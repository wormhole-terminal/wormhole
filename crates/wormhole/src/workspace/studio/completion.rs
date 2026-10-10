//! The expression field's completions: while a name is typed, the token names and
//! functions that fit it (core's `completions`), in a list under the field. ↑ ↓ move along it,
//! ⇥ and ⏎ take the highlighted one, Esc closes the list (and only then, pressed again, the
//! window). Clicking a row takes it without moving the keyboard out of the field. Taking a name
//! puts the caret after it, and the field's own Change then offers what follows (`primary.`
//! goes on to `primary.hover`) or, once the name is a color that exists, nothing: Return applies.

use std::ops::Range;

use gpui_kit::component::input::{Escape, IndentInline, MoveDown, MoveUp};
use wormhole_core::tokens::{Completion, completions};

use super::*;

/// The list while it is open.
pub(super) struct Open {
    /// The bytes of the field's text the taken name replaces: the whole name the caret is in.
    range: Range<usize>,
    pub(super) items: Vec<Completion>,
    pub(super) highlighted: usize,
}

impl StudioWindow {
    /// After every change of the field: what fits the name at the caret, or nothing (a metric
    /// takes a number, so never a list).
    pub(super) fn refresh_completions(&mut self, cx: &mut Context<Self>) {
        let target = self.selected.as_deref().filter(|name| !is_metric(name));
        let (text, caret) = {
            let input = self.expression.read(cx);
            (input.value().to_string(), input.cursor())
        };
        self.completion = target.and_then(|name| completions(&text, caret, name)).map(|c| Open {
            range: c.range,
            items: c.items,
            highlighted: 0,
        });
        cx.notify();
    }

    pub(super) fn close_completions(&mut self, cx: &mut Context<Self>) {
        if self.completion.take().is_some() {
            cx.notify();
        }
    }

    /// ↑ ↓: the highlight moves along the list and round its ends.
    pub(super) fn step_completion(&mut self, delta: isize, cx: &mut Context<Self>) {
        if let Some(open) = &mut self.completion
            && let Ok(count) = isize::try_from(open.items.len())
            && count > 0
        {
            let at = isize::try_from(open.highlighted).unwrap_or(0);
            open.highlighted = usize::try_from((at + delta).rem_euclid(count)).unwrap_or(0);
            cx.notify();
        }
    }

    /// Take the `ix`th name (the highlighted one without), in place of the one typed, with the
    /// caret after it: one undo step, and a Change the field reports like any typing.
    pub(super) fn take_completion(&mut self, ix: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.completion.take() else { return };
        let Some(item) = open.items.get(ix.unwrap_or(open.highlighted)) else { return };
        let text = item.insert.clone();
        self.expression.update(cx, |input, cx| {
            input.set_selected_range(open.range, cx);
            input.replace(text, window, cx);
        });
        cx.notify();
    }

    /// The keys the list takes while it is open, on the field's wrapper, ahead of the window's
    /// own (an Esc with the list open closes the list, not the window). ⏎ arrives as the field's
    /// `PressEnter`. With no list, each goes on to whoever had it.
    pub(super) fn completion_keys(&self, wrapper: Div, cx: &mut Context<Self>) -> Div {
        wrapper
            .on_action(cx.listener(|this, _: &MoveUp, _, cx| {
                if this.completion.is_some() { this.step_completion(-1, cx) } else { cx.propagate() }
            }))
            .on_action(cx.listener(|this, _: &MoveDown, _, cx| {
                if this.completion.is_some() { this.step_completion(1, cx) } else { cx.propagate() }
            }))
            .on_action(cx.listener(|this, _: &IndentInline, window, cx| {
                if this.completion.is_some() {
                    this.take_completion(None, window, cx)
                } else {
                    cx.propagate()
                }
            }))
            .on_action(cx.listener(|this, _: &Escape, _, cx| {
                if this.completion.is_some() { this.close_completions(cx) } else { cx.propagate() }
            }))
    }

    /// The list under the field: each name in the code font, with its group (a function says
    /// so) faint on the right; the highlighted row in the selection tint.
    pub(super) fn render_completions(
        &self,
        code_font: SharedString,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let open = self.completion.as_ref()?;
        let rows = open.items.iter().enumerate().map(|(ix, item)| {
            let detail = if item.function { "function" } else { group_of(&item.label).unwrap_or_default() };
            style::row(SharedString::from(format!("studio-completion-{ix}")), ui, ix == open.highlighted)
                .text_caption()
                // The field keeps the keyboard: the root would take it on this press.
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, window, cx| {
                        window.prevent_default();
                        this.take_completion(Some(ix), window, cx);
                    }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_family(code_font.clone())
                        .child(item.label.clone()),
                )
                .child(div().flex_none().text_color(ui.faint).child(detail))
        });
        Some(
            div()
                .id("studio-completions")
                .flex()
                .flex_col()
                .p_1()
                .rounded(ui.radius)
                .outlined(ui, ui.border)
                .bg(ui.elevated)
                .children(rows)
                .into_any_element(),
        )
    }
}
