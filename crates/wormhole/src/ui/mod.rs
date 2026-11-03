//! Components compiled from the design's markup.
//!
//! Each component is written once as markup in the design's own tooling, which turns it into the GPUI code
//! in `generated/` (committed, and never edited by hand). So a window drawn from a component matches its
//! design from one source, and what the code adds is behaviour: the props a component is built from carry
//! its text, its state and its handlers; its controls arrive as slots.
//!
//! This module only gathers the generated components and the types they share. It uses the theme tokens
//! and the code font and nothing of `style`, which draws with it.

#[rustfmt::skip]
#[allow(clippy::all, reason = "generated code")]
mod generated;
mod drawn;
mod rule;

use std::rc::Rc;

use gpui_kit::{App, ClickEvent, Window};
use wormhole_core::theme::tokens::ThemeTokens;

use crate::settings::SettingsStore;

pub(crate) use generated::checkbox_control::CheckboxControlProps;
pub(crate) use generated::form_button::FormButtonProps;
pub(crate) use generated::menu_face::MenuFaceProps;
pub(crate) use generated::menu_item::MenuItemProps;
pub(crate) use generated::menu_panel::MenuPanelProps;
pub(crate) use generated::stepper::StepperProps;
pub(crate) use generated::switch_control::SwitchControlProps;
pub(crate) use rule::{Rule, Sides};

/// A click handler in a component's props. A `Rc` because a component's element tree is built again
/// every frame and each element that clicks takes its own copy.
pub(crate) type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// A state a component is drawn in whatever the pointer does: the component sheet (`settings-components`)
/// shows hover, pressed and focus this way. In the app it is always `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Force {
    #[default]
    None,
    Hover,
    Focus,
}

/// What a component is drawn in: the theme's colours (a component names them by token) and the code font.
pub(crate) struct Look<'a> {
    pub tokens: &'a ThemeTokens,
    /// The Borders setting (Increase Contrast turns it on): see `rule`.
    pub borders: bool,
    pub force: Force,
}

impl<'a> Look<'a> {
    /// The app's own theme, as the windows are drawn in.
    pub(crate) fn of(cx: &'a App) -> Self {
        Self::with_tokens(&cx.global::<SettingsStore>().tokens, cx)
    }

    /// Another theme's colours (the Theme Studio's draft) in the app's code font.
    pub(crate) fn with_tokens(tokens: &'a ThemeTokens, cx: &App) -> Self {
        Self { tokens, borders: crate::settings::ui(cx).borders, force: Force::None }
    }
}
