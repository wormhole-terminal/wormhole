//! Opening a terminal with a shell other than the default: the menu of the chevron beside the +
//! of the Terminals title and of the tab bars (and under a right-click on the + itself), as
//! Terminal.app's "New Tab with Profile". The shells on offer, their
//! keys and what each runs are core's `shells`; the Shell menu's "New Terminal With ▸" and the
//! palette's "New terminal: fish" entries go through the same method.

use std::rc::Rc;

use gpui_kit::component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenu, PopupMenuItem};
use wormhole_core::shells::{self, ShellChoice};

use super::focus::MenuBuilder;
use super::*;
use crate::settings::Settings;

/// The menu's heading, and the chevron's tooltip.
const MENU_TITLE: &str = "New terminal with";
const MENU_TITLE_TOOLTIP: &str = "New terminal with…";
/// The chevron's face, as narrow as the Open in… one.
const CHEVRON_W: f32 = 16.0;

/// The key `default_shell` names, as the choices spell it: what a plain + opens.
pub(super) fn default_key(settings: &Settings) -> &str {
    settings.default_shell.trim()
}

impl Workspace {
    /// `button` (a +) with the shells' menu under a right-click: every shell by name, the default
    /// (what a left click opens) checked.
    pub(super) fn with_shell_menu(
        &self,
        id: &'static str,
        button: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let build = self.shell_menu(cx);
        div()
            .id(id)
            .flex_none()
            .child(button)
            .context_menu(move |menu, window, cx| build(menu, window, cx))
            .into_any_element()
    }

    /// The chevron beside a + that opens the shells' menu on a click, so the other shells are
    /// in sight, not only under a right-click; none while there is only one shell. `anchor` is the
    /// menu's corner that meets the chevron.
    pub(super) fn shell_chevron(
        &self,
        id: &'static str,
        anchor: Anchor,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if shells::choices(settings(cx)).len() < 2 {
            return None;
        }
        let menu = self.shell_menu(cx);
        Some(
            div()
                .id(id)
                .occlude()
                .flex_none()
                .child(
                    style::icon_button(
                        SharedString::from(format!("{id}-button")),
                        ui(cx),
                        IconName::ChevronDown,
                        MENU_TITLE_TOOLTIP,
                    )
                    .narrow(CHEVRON_W)
                    .menu_trigger()
                    .dropdown_menu_with_anchor(anchor, move |m, w, c| menu(m, w, c)),
                )
                .into_any_element(),
        )
    }

    /// What builds the shells' menu.
    fn shell_menu(&self, cx: &mut Context<Self>) -> MenuBuilder {
        let settings = settings(cx);
        let choices: Vec<ShellChoice> = shells::choices(settings);
        let default = default_key(settings).to_string();
        let this = cx.weak_entity();
        Rc::new(move |mut menu: PopupMenu, _: &mut Window, _: &mut Context<PopupMenu>| {
            menu = menu.label(MENU_TITLE);
            for choice in &choices {
                let (this, key) = (this.clone(), choice.key.clone());
                menu = menu.item(
                    PopupMenuItem::new(choice.name.clone()).checked(choice.key == default).on_click(
                        move |_, window, cx| {
                            let key = key.clone();
                            this.update(cx, |ws, cx| ws.new_session_with(Some(key), window, cx)).ok();
                        },
                    ),
                );
            }
            menu
        })
    }
}
