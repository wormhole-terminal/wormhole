//! Host AppKit switches in the settings form while GPUI keeps the tab order and accessibility tree.
use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::theme::tokens::Appearance;

use super::SettingsWindow;
use crate::platform::controls::Switches;
use crate::style::native_switch::{NativeCheckbox, NativeSwitch};

pub(super) type Change = crate::style::native_switch::Change;

/// A sheet owns only its own permission controls; an action from a previous sheet must not
/// affect a newly opened approval, even if the same permission occupies the same row.
pub(super) fn scope_allows(sheet: Option<u64>, id: &str) -> bool {
    let permission = id.strip_prefix("probe-grant-");
    match (sheet, permission) {
        (None, None) => true,
        (Some(current), Some(rest)) => {
            rest.split_once('-').and_then(|(serial, _)| serial.parse::<u64>().ok()) == Some(current)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::scope_allows;

    #[test]
    fn approval_switch_actions_belong_to_the_sheet_that_created_them() {
        assert!(scope_allows(Some(4), "probe-grant-4-0"));
        assert!(!scope_allows(Some(5), "probe-grant-4-0"));
        assert!(!scope_allows(None, "probe-grant-4-0"));
        assert!(!scope_allows(Some(4), "ordinary-setting"));
        assert!(scope_allows(None, "ordinary-setting"));
    }
}

/// First and last prepaint probes bracket the rows, so rows that disappeared are hidden.
pub(super) fn begin(native: &Rc<Switches>) -> AnyElement {
    let native = native.clone();
    canvas(move |_, _, _| native.begin(), |_, _, _, _| {}).absolute().size_0().into_any_element()
}

pub(super) fn finish(native: &Rc<Switches>) -> AnyElement {
    let native = native.clone();
    canvas(move |_, _, _| native.finish(), |_, _, _, _| {}).absolute().size_0().into_any_element()
}

pub(super) fn switch(
    native: Rc<Switches>,
    id: String,
    label: SharedString,
    on: bool,
    enabled: bool,
    focus: FocusHandle,
    appearance: Appearance,
    change: Change,
) -> AnyElement {
    NativeSwitch { native, id, label, on, enabled, focus: Some(focus), appearance, change }.into_any_element()
}

impl SettingsWindow {
    pub(super) fn checkbox_control(
        &self,
        id: SharedString,
        label: impl Into<SharedString>,
        on: bool,
        enabled: bool,
        show_label: bool,
        cx: &mut Context<Self>,
        change: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> AnyElement {
        let label = label.into();
        if self.native_switch_allowed(&id, cx) {
            if let Some(native) = &self.native_switches {
                let change: Change = Rc::new(change);
                let id = id.to_string();
                let focus = self
                    .native_switch_focus
                    .borrow_mut()
                    .entry(id.clone())
                    .or_insert_with(|| cx.focus_handle())
                    .clone();
                self.native_switch_handlers.borrow_mut().insert(id.clone(), change.clone());
                return NativeCheckbox {
                    native: native.clone(),
                    id,
                    label,
                    on,
                    enabled,
                    show_label,
                    focus: Some(focus),
                    change,
                    appearance: cx.global::<crate::settings::SettingsStore>().tokens.appearance,
                    ui: super::ui(cx),
                }
                .into_any_element();
            }
        }
        let checkbox = crate::style::checkbox(id, super::ui(cx), on);
        let checkbox =
            if show_label { checkbox.in_form(label) } else { checkbox.spoken(label).tab_stop(true) };
        checkbox.disabled(!enabled).on_change(change).into_any_element()
    }

    pub(super) fn forget_approval_switches(&self, serial: u64) {
        let prefix = format!("probe-grant-{serial}-");
        self.native_switch_focus.borrow_mut().retain(|id, _| !id.starts_with(&prefix));
        self.native_switch_handlers.borrow_mut().retain(|id, _| !id.starts_with(&prefix));
        if let Some(native) = &self.native_switches {
            native.forget_prefix(&prefix);
        }
    }

    /// One switch boundary for both registry rows and the settings' specialized controls.
    pub(super) fn switch_control(
        &self,
        id: SharedString,
        label: impl Into<SharedString>,
        on: bool,
        enabled: bool,
        cx: &mut Context<Self>,
        change: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> AnyElement {
        let label = label.into();
        if self.native_switch_allowed(&id, cx) {
            if let Some(native) = &self.native_switches {
                let change: Change = Rc::new(change);
                let id = id.to_string();
                let focus = self
                    .native_switch_focus
                    .borrow_mut()
                    .entry(id.clone())
                    .or_insert_with(|| cx.focus_handle())
                    .clone();
                self.native_switch_handlers.borrow_mut().insert(id.clone(), change.clone());
                return switch(
                    native.clone(),
                    id,
                    label,
                    on,
                    enabled,
                    focus,
                    cx.global::<crate::settings::SettingsStore>().tokens.appearance,
                    change,
                );
            }
        }
        use gpui_kit::component::Disableable as _;
        crate::style::toggle(id, on, change).disabled(!enabled).accessibility_label(label).into_any_element()
    }
}
