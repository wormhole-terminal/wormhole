//! Settings-owned native slider host.
use super::SettingsWindow;
use crate::platform::controls::Sliders;
use crate::settings::SettingsStore;
use crate::style::native_slider::NativeSlider;
use gpui_kit::*;
use std::rc::Rc;

pub(super) type Change = crate::style::native_slider::Change;

pub(super) fn begin(native: &Rc<Sliders>) -> AnyElement {
    let native = native.clone();
    canvas(move |_, _, _| native.begin(), |_, _, _, _| {}).absolute().size_0().into_any_element()
}
pub(super) fn finish(native: &Rc<Sliders>) -> AnyElement {
    let native = native.clone();
    canvas(move |_, _, _| native.finish(), |_, _, _, _| {}).absolute().size_0().into_any_element()
}

pub(super) fn control(
    owner: &SettingsWindow,
    id: &'static str,
    label: impl Into<SharedString>,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    enabled: bool,
    cx: &mut Context<SettingsWindow>,
    change: impl Fn(f64, &mut Window, &mut App) + 'static,
) -> Option<AnyElement> {
    if !owner.native_switch_allowed(id, cx) {
        return None;
    }
    let native = owner.native_sliders.as_ref()?.clone();
    let focus = owner
        .native_slider_focus
        .borrow_mut()
        .entry(id.to_owned())
        .or_insert_with(|| cx.focus_handle())
        .clone();
    let change: Change = Rc::new(change);
    owner.native_slider_handlers.borrow_mut().insert(id.to_owned(), change.clone());
    Some(
        NativeSlider {
            native,
            id: id.to_owned(),
            label: label.into(),
            value,
            min,
            max,
            step,
            enabled,
            focus,
            appearance: cx.global::<SettingsStore>().tokens.appearance,
            change,
        }
        .into_any_element(),
    )
}
