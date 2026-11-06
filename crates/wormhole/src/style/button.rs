//! The button of the Settings window and its sheets: GPUI Kit's `Button`, drawn flat as System Settings
//! draws one. The pages build it with the Kit's own vocabulary (`.primary()`, `.outline()`, `.ghost()`,
//! `.danger()`, `.small()`, `.label(…)`), so a page changes its import and nothing else; the look
//! comes from the design tokens when it is drawn, which is when the window's colours are known.
//!
//! | Kit call | Look |
//! |---|---|
//! | `.primary()` | the primary's fill, `text.on_primary`, the control corners, no outline |
//! | none, `.outline()`, `.secondary()` | a flat `control.background` fill (`control.hover` under the pointer), `text.primary`, no outline |
//! | `.ghost()` | the label alone, a hover fill |
//! | `.danger()` | the danger colour's tint (10 %, 20 % under the pointer) with its text, no outline |
//! | `.link()`, `.text()` and the rest | the Kit's own |
//!
//! Focus, `.loading(true)` and `.disabled(true)` are the Kit's. Increase Contrast gives a flat button its
//! outline back, since a faint fill alone is no edge; Borders off changes nothing, there is no outline
//! at rest to take away.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button as KitButton, ButtonCustomVariant, ButtonVariant, ButtonVariants};
use gpui_kit::component::{Disableable, Selectable, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::Edges;
use crate::settings::{self, UiColors};

/// A Kit button with Wormhole's flat look. See the module for what each variant draws.
#[derive(IntoElement)]
pub struct Button {
    inner: KitButton,
    variant: ButtonVariant,
    outline: bool,
}

impl Button {
    #[track_caller]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { inner: KitButton::new(id), variant: ButtonVariant::Default, outline: false }
    }

    /// An outlined button in the Kit; here the same as a plain one, which is flat already.
    #[must_use]
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self.inner = self.inner.outline();
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.inner = self.inner.label(label);
        self
    }

    #[must_use]
    pub fn icon(mut self, icon: IconName) -> Self {
        self.inner = self.inner.icon(icon);
        self
    }

    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.inner = self.inner.tooltip(tooltip);
        self
    }

    #[must_use]
    pub fn loading(mut self, loading: bool) -> Self {
        self.inner = self.inner.loading(loading);
        self
    }

    #[must_use]
    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.inner = self.inner.on_click(handler);
        self
    }
}

impl ButtonVariants for Button {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self.inner = self.inner.with_variant(variant);
        self
    }
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<gpui_kit::component::Size>) -> Self {
        self.inner = self.inner.with_size(size);
        self
    }
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.inner = self.inner.disabled(disabled);
        self
    }
}

impl Selectable for Button {
    fn selected(mut self, selected: bool) -> Self {
        self.inner = self.inner.selected(selected);
        self
    }

    fn is_selected(&self) -> bool {
        self.inner.is_selected()
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(elements);
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let ui = settings::ui(cx);
        let Button { inner, variant, outline } = self;
        let inner = inner.rounded(ui.radius);
        match variant {
            ButtonVariant::Primary if !outline => inner,
            ButtonVariant::Default | ButtonVariant::Secondary | ButtonVariant::Primary => flat(inner, ui, cx),
            ButtonVariant::Danger => tinted_danger(inner, ui),
            _ => inner,
        }
    }
}

/// The secondary button: a flat `control.background` fill, which the Kit's custom variant takes for its
/// hover and pressed colours; the resting fill is the button's own style, because the variant mixes its
/// resting colour toward transparent.
fn flat(inner: KitButton, ui: UiColors, cx: &App) -> KitButton {
    let clear = transparent_black();
    let look = ButtonCustomVariant::new(cx)
        .color(clear)
        .hover(ui.control_hover)
        .active(ui.control_hover)
        .foreground(ui.text);
    inner
        .custom(look)
        .bg(ui.control)
        .when(ui.increase_contrast, |button| button.outlined_always(ui, ui.border_strong))
}

/// The danger button: the Kit's tinted (outline) danger without its outline.
fn tinted_danger(inner: KitButton, ui: UiColors) -> KitButton {
    inner
        .outline()
        .border_0()
        .when(ui.increase_contrast, |button| button.outlined_always(ui, ui.banner_danger_border))
}
