//! Native steppers with a GPUI value, keyboard focus, and accessible increment/decrement actions.
use super::SettingsWindow;
use crate::platform::controls::Steppers;
use crate::settings::{SettingsStore, UiColors};
use crate::style::native_stepper::NativeStepper;
use gpui_kit::*;
use std::rc::Rc;

pub(super) type Change = crate::style::native_stepper::Change;

pub(super) fn begin(native: &Rc<Steppers>) -> AnyElement {
    let native = native.clone();
    canvas(move |_, _, _| native.begin(), |_, _, _, _| {}).absolute().size_0().into_any_element()
}
pub(super) fn finish(native: &Rc<Steppers>) -> AnyElement {
    let native = native.clone();
    canvas(move |_, _, _| native.finish(), |_, _, _, _| {}).absolute().size_0().into_any_element()
}

impl SettingsWindow {
    pub(super) fn stepper_control(
        &self,
        ui: UiColors,
        id: &'static str,
        label: impl Into<SharedString>,
        value: String,
        enabled: bool,
        cx: &mut Context<Self>,
        change: impl Fn(i32, &mut Window, &mut App) + 'static,
    ) -> AnyElement {
        let label = label.into();
        if self.native_switch_allowed(id, cx) {
            if let Some(native) = &self.native_steppers {
                let change: Change = Rc::new(change);
                let focus = self
                    .native_stepper_focus
                    .borrow_mut()
                    .entry(id.to_owned())
                    .or_insert_with(|| cx.focus_handle())
                    .clone();
                self.native_stepper_handlers.borrow_mut().insert(id.to_owned(), change.clone());
                return NativeStepper {
                    native: native.clone(),
                    id: id.to_string(),
                    label,
                    value,
                    enabled,
                    focus,
                    appearance: cx.global::<SettingsStore>().tokens.appearance,
                    ui,
                    change,
                }
                .into_any_element();
            }
        }
        let change: Change = Rc::new(change);
        let minus = change.clone();
        crate::style::stepper(
            ui,
            id,
            value,
            move |_, window, cx| minus(-1, window, cx),
            move |_, window, cx| change(1, window, cx),
        )
        .into_any_element()
    }
}
