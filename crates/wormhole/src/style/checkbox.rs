//! The checkbox: a next step's box in the panel's lead column and the
//! form control of a settings row. GPUI Kit's own reads `input` for its unchecked outline, which
//! is 1.55:1 in Catppuccin Latte; a 3:1 outline needs `checkbox.border`, and recolouring `input` would
//! move every Kit field. So the box is drawn here, on the Kit's base checkbox, which brings the
//! role, the checked state VoiceOver reads, and ↩ / Space.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{Checkbox as BaseCheckbox, CheckboxIndicator};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::controls::{RingPlacement, focus_ring};
use super::scale::{Edges, HIT_MIN, TextRole};
use crate::settings::UiColors;

/// The box in a panel's lead column and in a form.
const BOX: f32 = 14.0;
const BOX_FORM: f32 = 16.0;
/// The check mark inside it.
const MARK: f32 = 10.0;
const MARK_FORM: f32 = 12.0;

/// What a click on a [`Checkbox`] does: told the new state.
type OnChange = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A checkbox: the box in `checkbox.background` with a `checkbox.border` outline, filled with
/// `checkbox.checked` and a check in `text.on_primary` when on, the chip corners. It takes clicks
/// across [`HIT_MIN`] round the box without taking the room (a panel row's lead column), or, in a
/// form, across its label too. Build it with [`checkbox`].
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    ui: UiColors,
    checked: bool,
    form: bool,
    label: Option<SharedString>,
    spoken: Option<SharedString>,
    on_change: Option<OnChange>,
    ring: bool,
}

/// A checkbox the size of a panel's lead column (14 pt). Add `.on_change(…)`.
pub fn checkbox(id: impl Into<ElementId>, ui: UiColors, checked: bool) -> Checkbox {
    Checkbox {
        id: id.into(),
        ui,
        checked,
        form: false,
        label: None,
        spoken: None,
        on_change: None,
        ring: false,
    }
}

impl Checkbox {
    /// A form's checkbox (16 pt) with its label after it, which VoiceOver reads too.
    pub fn in_form(mut self, label: impl Into<SharedString>) -> Self {
        self.form = true;
        self.label = Some(label.into());
        self
    }

    /// What VoiceOver calls a checkbox without a label of its own (a next step: its text).
    pub fn spoken(mut self, text: impl Into<SharedString>) -> Self {
        self.spoken = Some(text.into());
        self
    }

    pub fn on_change(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }

    /// Draw the keyboard focus ring round the box (outside, with a gap).
    pub fn focus_ring(mut self, shown: bool) -> Self {
        self.ring = shown;
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let Checkbox { id, ui, checked, form, label, spoken, on_change, ring } = self;
        let (side, mark) = if form { (BOX_FORM, MARK_FORM) } else { (BOX, MARK) };
        let indicator = CheckboxIndicator::new()
            .checked(checked)
            .relative()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(side))
            .rounded(ui.radius_chip)
            .bg(ui.checkbox_bg)
            .outlined(ui, ui.checkbox_border)
            .styles(|styles| {
                styles.checked(|style| style.bg(ui.checkbox_checked).border_color(ui.checkbox_checked))
            })
            .when(checked, |d| {
                d.child(Icon::new(IconName::Check).with_size(px(mark)).text_color(ui.on_primary))
            })
            .when(ring, |d| d.child(focus_ring(ui, RingPlacement::Outside, ui.radius_chip)));
        let spoken = label.clone().or(spoken);
        let base = BaseCheckbox::new(id)
            .checked(checked)
            // The workspace moves the keyboard through its lists itself; a box of its own
            // would be one more stop in every row.
            .tab_stop(false)
            .when_some(spoken, |d, text| d.accessibility_label(text))
            .when_some(on_change, |d, on_change| {
                d.on_change(move |_, _, window, cx| {
                    cx.stop_propagation();
                    on_change(!checked, window, cx);
                })
            })
            .flex()
            .flex_none()
            .items_center()
            .cursor_pointer();
        match label {
            // In a form the label takes the clicks too.
            Some(label) => base
                .gap_2()
                .child(indicator)
                .child(div().text_body().text_color(ui.text).child(label))
                .into_any_element(),
            // In a lead column the square round the box takes the clicks and hangs out
            // over its neighbors: the row lays out a slot the box's size (see `hit_area`).
            None => {
                let overhang = (HIT_MIN - side) / 2.0;
                div()
                    .flex_none()
                    .size(px(side))
                    .child(base.justify_center().size(px(HIT_MIN)).m(px(-overhang)).child(indicator))
                    .into_any_element()
            }
        }
    }
}

#[cfg(all(test, feature = "devctl"))]
mod window_tests {
    use gpui_kit::prelude::*;
    use gpui_kit::{
        Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowOptions, div, px,
    };

    use super::checkbox;
    use crate::settings::{SettingsStore, ui};

    /// A next step's row (a 10 pt lead-in, the box, its text) and a form's checkbox with a label,
    /// followed by what comes next on its row.
    struct Rows;

    impl Render for Rows {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let ui = ui(cx);
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .child(div().w(px(10.0)).h(px(20.0)))
                        .child(checkbox("lead", ui, false))
                        .child(div().debug_selector(|| "after-lead".into()).child("Ship it")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .child(checkbox("form", ui, false).in_form("Theme Studio"))
                        .child(div().debug_selector(|| "after-form".into()).child("next")),
                )
        }
    }

    /// The box keeps its place in the row (the square round it hangs out without taking room),
    /// and a form's label takes its room.
    #[gpui_kit::test]
    fn a_checkbox_takes_the_room_it_draws(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_global(SettingsStore::for_test());
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Rows))
                .expect("opens the window")
        });
        let mut drawn = VisualTestContext::from_window(window.0, cx);
        drawn.run_until_parked();
        let next = drawn.debug_bounds("after-lead").expect("drawn");
        assert_eq!(next.origin.x, px(10.0 + super::BOX), "the box takes its own width, no more");
        let after_form = drawn.debug_bounds("after-form").expect("drawn");
        assert!(after_form.origin.x > px(super::BOX_FORM + 60.0), "the label has room: {after_form:?}");
    }
}
