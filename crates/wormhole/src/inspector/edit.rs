//! The style section of the panel: an element's box, spacing, shape, text and colors,
//! with steppers for the values worth nudging and the changes made so far. Each
//! stepper edits the element's style live through GPUI's inspector state.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::facts::{Facts, TEXT_ROLES, abs_px};
use super::sources::{element_name, short_file, source_line};
use crate::settings::{UiColors, ui};
use crate::style::{self, TextRole};

/// The style section for a `div`: its box, spacing, shape, text and colors, with
/// steppers for the values worth nudging and the changes made so far.
pub(super) fn render_div(
    id: InspectorElementId,
    state: &DivInspectorState,
    original: &mut Option<(InspectorElementId, StyleRefinement)>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let ui = ui(cx);
    let rem = f32::from(window.rem_size());
    super::skip_kit_layers(&id, state, window);
    #[cfg(feature = "devctl")]
    super::probe::record(&id, state, window);
    if original.as_ref().is_none_or(|(seen, _)| seen != &id) {
        *original = Some((id.clone(), (*state.base_style).clone()));
    }
    let before = original.as_ref().map(|(_, s)| s.clone()).unwrap_or_default();
    let now = &*state.base_style;
    let f = Facts::of(now, ui, rem);
    let changes = Facts::of(&before, ui, rem).changes_to(&f);

    let size = state.bounds.size;
    let summary = format!(
        "{:.0}×{:.0} · padding {} · gap {} · radius {} · text {}",
        f32::from(size.width),
        f32::from(size.height),
        f.padding,
        f.gap,
        f.radius,
        f.text
    );
    let location = id.path.source_location;
    let mut note = format!("{}:{}  {}", short_file(location.file()), location.line(), element_name(&id));
    if let Some(code) = source_line(location.file(), location.line()) {
        note.push_str(&format!("\n  {code}"));
    }
    note.push_str(&format!("\n  {summary}"));
    if !changes.is_empty() {
        note.push_str(&format!("\n  change: {}", changes.join("; ")));
    }
    super::NOTE.with(|n| *n.borrow_mut() = Some((id.clone(), note)));

    let edit = |key: &'static str,
                label: &'static str,
                value: String,
                change: fn(&mut StyleRefinement, f32, f32)| {
        let (minus, plus) = (id.clone(), id.clone());
        let apply = move |target: InspectorElementId, dir: f32| {
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                let rem = f32::from(window.rem_size());
                window.with_inspector_state::<DivInspectorState, _>(Some(&target), cx, |state, _| {
                    if let Some(state) = state {
                        change(&mut state.base_style, dir, rem);
                    }
                });
                window.refresh();
            }
        };
        prop(ui, label, value).child(
            div()
                .flex()
                .flex_none()
                .gap_1()
                .child(
                    Button::new((key, 0usize))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Minus)
                        .on_click(apply(minus, -1.0)),
                )
                .child(
                    Button::new((key, 1usize))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Plus)
                        .on_click(apply(plus, 1.0)),
                ),
        )
    };

    let reset = {
        let (target, before) = (id.clone(), before.clone());
        move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            window.with_inspector_state::<DivInspectorState, _>(Some(&target), cx, |state, _| {
                if let Some(state) = state {
                    *state.base_style = before.clone();
                }
            });
            window.refresh();
        }
    };

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(style::section_label(ui, "Box"))
                .child(prop(
                    ui,
                    "size",
                    format!("{:.0} × {:.0}", f32::from(size.width), f32::from(size.height)),
                ))
                .child(edit("insp-h", "height", f.height.clone(), |s, dir, _| {
                    s.size.height = Some(step_length(s.size.height, dir));
                }))
                .child(prop(ui, "min height", f.min_height.clone())),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(style::section_label(ui, "Spacing"))
                .child(prop(ui, "padding", f.padding.clone()))
                .child(edit("insp-px", "padding x", f.padding_x.clone(), |s, dir, _| {
                    s.padding.left = Some(step_spacing(s.padding.left, dir));
                    s.padding.right = Some(step_spacing(s.padding.right, dir));
                }))
                .child(edit("insp-py", "padding y", f.padding_y.clone(), |s, dir, _| {
                    s.padding.top = Some(step_spacing(s.padding.top, dir));
                    s.padding.bottom = Some(step_spacing(s.padding.bottom, dir));
                }))
                .child(edit("insp-gap", "gap", f.gap.clone(), |s, dir, _| {
                    s.gap.width = Some(step_spacing(s.gap.width, dir));
                    s.gap.height = Some(step_spacing(s.gap.height, dir));
                })),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(style::section_label(ui, "Shape & text"))
                .child(edit("insp-radius", "radius", f.radius.clone(), |s, dir, rem| {
                    let now = s.corner_radii.top_left.map_or(0.0, |r| abs_px(r, rem));
                    let next = px((now + 2.0 * dir).max(0.0));
                    s.corner_radii.top_left = Some(next.into());
                    s.corner_radii.top_right = Some(next.into());
                    s.corner_radii.bottom_left = Some(next.into());
                    s.corner_radii.bottom_right = Some(next.into());
                }))
                .child(prop(ui, "border", f.border.clone()))
                .child(edit("insp-text", "text", f.text.clone(), |s, dir, rem| {
                    let now = s.text.font_size.map_or(TEXT_ROLES[1].1, |size| abs_px(size, rem) / rem);
                    let ix = TEXT_ROLES.iter().position(|(_, r)| (r - now).abs() < 0.01);
                    let next = match (ix, dir > 0.0) {
                        (Some(ix), true) => (ix + 1).min(TEXT_ROLES.len() - 1),
                        (Some(ix), false) => ix.saturating_sub(1),
                        (None, _) => 1,
                    };
                    s.text.font_size = Some(rems(TEXT_ROLES[next].1).into());
                })),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(style::section_label(ui, "Colors"))
                .child(prop(ui, "background", f.background.clone()))
                .child(prop(ui, "text color", f.text_color.clone()))
                .child(prop(ui, "border color", f.border_color.clone())),
        )
        .when(!changes.is_empty(), |d| {
            d.child(
                style::card(ui, ui.primary)
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(div().flex_1().child(style::section_label(ui, "Changes")))
                            .child(Button::new("insp-reset").ghost().xsmall().label("Reset").on_click(reset)),
                    )
                    .children(changes.iter().map(|c| div().text_caption().child(c.clone()))),
            )
        })
        .into_any_element()
}

/// One labelled value in the panel.
fn prop(ui: UiColors, label: &'static str, value: String) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .min_h(px(style::CONTROL_H_SM))
        .child(div().w(px(96.0)).flex_none().text_caption().text_color(ui.muted).child(label))
        .child(div().flex_1().min_w_0().text_caption().truncate().child(value))
}

/// One grid step (4 pt) more or less, keeping the value's unit.
fn step_spacing(len: Option<DefiniteLength>, dir: f32) -> DefiniteLength {
    match len {
        Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(p))) => {
            px((f32::from(p) + 4.0 * dir).max(0.0)).into()
        }
        Some(DefiniteLength::Fraction(f)) => DefiniteLength::Fraction(f),
        Some(DefiniteLength::Absolute(AbsoluteLength::Rems(r))) => rems((r.0 + 0.25 * dir).max(0.0)).into(),
        None => rems((0.25 * dir).max(0.0)).into(),
    }
}

fn step_length(len: Option<Length>, dir: f32) -> Length {
    match len {
        Some(Length::Definite(d)) => Length::Definite(step_spacing(Some(d), dir)),
        // No height set yet: start from nothing rather than guess the laid-out one.
        _ => Length::Definite(px((4.0 * dir).max(0.0)).into()),
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{AbsoluteLength, DefiniteLength, Length, px, rems};

    use super::{step_length, step_spacing};

    fn rem_len(steps: f32) -> Option<DefiniteLength> {
        Some(DefiniteLength::Absolute(AbsoluteLength::Rems(rems(steps))))
    }

    #[test]
    fn spacing_steps_one_grid_unit_keeping_its_unit() {
        assert_eq!(Some(step_spacing(rem_len(0.5), 1.0)), rem_len(0.75));
        assert_eq!(Some(step_spacing(rem_len(0.5), -1.0)), rem_len(0.25));
        assert_eq!(
            step_spacing(Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(8.0)))), 1.0),
            px(12.0).into()
        );
        assert_eq!(step_spacing(Some(DefiniteLength::Fraction(0.5)), 1.0), DefiniteLength::Fraction(0.5));
    }

    #[test]
    fn spacing_never_goes_below_zero() {
        assert_eq!(Some(step_spacing(rem_len(0.0), -1.0)), rem_len(0.0));
        assert_eq!(Some(step_spacing(None, -1.0)), rem_len(0.0));
        assert_eq!(Some(step_spacing(None, 1.0)), rem_len(0.25));
        assert_eq!(
            step_spacing(Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(2.0)))), -1.0),
            px(0.0).into()
        );
    }

    #[test]
    fn an_unset_height_starts_from_nothing() {
        assert_eq!(step_length(None, 1.0), Length::Definite(px(4.0).into()));
        assert_eq!(step_length(Some(Length::Auto), -1.0), Length::Definite(px(0.0).into()));
        assert_eq!(
            step_length(Some(Length::Definite(rem_len(1.0).unwrap())), 1.0),
            Length::Definite(rem_len(1.25).unwrap())
        );
    }
}
