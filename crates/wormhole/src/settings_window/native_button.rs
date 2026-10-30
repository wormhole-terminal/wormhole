//! Settings-owned AppKit buttons, with focus and overlay guards.
use super::SettingsWindow;
use crate::platform::controls::Buttons;
use crate::style::native_button::NativeButton;
use gpui_kit::*;
use std::rc::Rc;

pub(super) type Press = Rc<dyn Fn(&mut Window, &mut App)>;

pub(super) fn begin(host: &Rc<Buttons>) -> AnyElement {
    let host = host.clone();
    canvas(move |_, _, _| host.begin(), |_, _, _, _| {}).absolute().size_0().into_any_element()
}
pub(super) fn finish(host: &Rc<Buttons>) -> AnyElement {
    let host = host.clone();
    canvas(move |_, _, _| host.finish(), |_, _, _, _| {}).absolute().size_0().into_any_element()
}

impl SettingsWindow {
    pub(super) fn native_button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        enabled: bool,
        cx: &mut Context<Self>,
        press: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Option<AnyElement> {
        self.native_button_with_style(id, label, enabled, false, cx, press)
    }

    pub(super) fn native_button_with_style(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        enabled: bool,
        primary: bool,
        cx: &mut Context<Self>,
        press: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Option<AnyElement> {
        self.native_button_with_tooltip(id, label, enabled, primary, None, cx, press)
    }

    pub(super) fn native_button_with_tooltip(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        enabled: bool,
        primary: bool,
        tooltip: Option<SharedString>,
        cx: &mut Context<Self>,
        press: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Option<AnyElement> {
        self.native_button_with_options(id, label, enabled, primary, false, tooltip, cx, press)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn native_button_with_options(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        enabled: bool,
        primary: bool,
        destructive: bool,
        tooltip: Option<SharedString>,
        cx: &mut Context<Self>,
        press: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Option<AnyElement> {
        let id = id.into();
        if !self.native_switch_allowed(&id, cx) {
            return None;
        }
        let native = self.native_buttons.as_ref()?.clone();
        let focus = self
            .native_button_focus
            .borrow_mut()
            .entry(id.to_string())
            .or_insert_with(|| cx.focus_handle())
            .clone();
        let press: Press = Rc::new(press);
        self.native_button_handlers.borrow_mut().insert(id.to_string(), press.clone());
        Some(
            NativeButton {
                native,
                id: id.to_string(),
                label: label.into(),
                tooltip,
                enabled,
                primary,
                destructive,
                focus,
                appearance: cx.global::<crate::settings::SettingsStore>().tokens.appearance,
                press,
            }
            .into_any_element(),
        )
    }

    pub(super) fn native_button_action(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Press> {
        if !self.native_switch_allowed(id, cx)
            || !self.native_buttons.as_ref().is_some_and(|host| host.accepts(id))
        {
            return None;
        }
        let press = self.native_button_handlers.borrow().get(id).cloned();
        if let Some(focus) = self.native_button_focus.borrow().get(id) {
            crate::style::native_focus::pointer(window);
            window.focus(focus, cx);
        }
        press
    }
}
