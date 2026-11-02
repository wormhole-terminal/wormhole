//! The text button of the Settings window: drawn by the `FormButton` component (the design's button, 24 tall,
//! three styles, a label and no icon). This is its builder: the call sites say what the button is, and the
//! look is the design's.

use std::rc::Rc;

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Disableable, Selectable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::ui::{FormButtonProps, Handler, Look};

/// A button. Pick its style with [`primary`](Self::primary) or [`danger`](Self::danger) (red text on the
/// normal fill); add the label and `.on_click(…)`.
#[derive(IntoElement)]
pub struct FormButton {
    id: ElementId,
    label: SharedString,
    primary: bool,
    destructive: bool,
    disabled: bool,
    on_click: Option<Handler>,
    tooltip: Option<SharedString>,
}

/// A text button in a form or a Settings row.
pub fn form_button(id: impl Into<ElementId>) -> FormButton {
    FormButton {
        id: id.into(),
        label: SharedString::default(),
        primary: false,
        destructive: false,
        disabled: false,
        on_click: None,
        tooltip: None,
    }
}

impl FormButton {
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    /// The destructive style: red text on the normal fill.
    pub fn danger(mut self) -> Self {
        self.destructive = true;
        self
    }

    /// A spinner is not part of the design: the button is off and shows its words, if any.
    pub fn loading(mut self, loading: bool) -> Self {
        self.disabled |= loading;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }
}

impl Disableable for FormButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for FormButton {
    fn selected(self, _: bool) -> Self {
        self
    }

    fn is_selected(&self) -> bool {
        false
    }
}

impl RenderOnce for FormButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { id, label, primary, destructive, disabled, on_click, tooltip } = self;
        FormButtonProps {
            id,
            label,
            primary,
            destructive,
            disabled,
            on_click: on_click.filter(|_| !disabled),
        }
        .render(&Look::of(cx))
        .when_some(tooltip, |button, text| {
            button.tooltip(move |window, cx| Tooltip::new(text.clone()).build(window, cx))
        })
    }
}
