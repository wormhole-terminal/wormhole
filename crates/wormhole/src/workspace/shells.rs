//! Opening a terminal with a shell other than the default: the menu under a right-click on the +
//! of the Terminals title and of the tab bar, as Terminal.app's "New Tab with Profile". The shells on offer, their
//! keys and what each runs are core's `shells`; the Shell menu's "New Terminal With ▸" and the
//! palette's "New terminal: fish" entries go through the same method.

use gpui_kit::*;
use wormhole_core::terminal::shells::{self, ShellChoice};

use crate::platform::{self, Item, Menu};
use crate::settings::Settings;
use crate::settings::settings;
use crate::workspace::Workspace;

/// The menu's heading.
const MENU_TITLE: &str = "New terminal with";

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
        let this = cx.weak_entity();
        div()
            .id(id)
            .relative()
            .flex_none()
            .debug_selector(move || id.to_string())
            .child(button)
            // Over the button, which blocks the pointer from what lies behind it.
            .child(platform::on_right_click(move |at, window, cx| {
                this.update(cx, |ws, cx| {
                    let menu = ws.shell_menu(cx);
                    platform::show_menu(menu, at, window, cx);
                })
                .ok();
            }))
            .into_any_element()
    }

    /// The shells' menu, with the shells and the default as they are now.
    fn shell_menu(&self, cx: &mut Context<Self>) -> Menu {
        let settings = settings(cx);
        shell_menu(&shells::choices(settings), default_key(settings), &cx.weak_entity())
    }
}

/// The menu of `choices`, `default` checked; choosing one opens a terminal with it in `workspace`.
fn shell_menu(choices: &[ShellChoice], default: &str, workspace: &WeakEntity<Workspace>) -> Menu {
    choices.iter().fold(Menu::new().header(MENU_TITLE), |menu, choice| {
        let key = choice.key.clone();
        menu.item(
            Item::new(choice.name.clone())
                .checked(choice.key == default)
                .on_select(platform::run_on(workspace, move |ws, window, cx| {
                    ws.new_session_with(Some(key.clone()), window, cx)
                })),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::shell_menu;
    use gpui_kit::WeakEntity;
    use wormhole_core::terminal::shells::{ShellChoice, ShellKind};

    fn choice(key: &str, name: &str) -> ShellChoice {
        ShellChoice { key: key.into(), name: name.into(), command_line: name.into(), kind: ShellKind::System }
    }

    /// The default shell is the checked one, under the heading.
    #[test]
    fn the_default_shell_is_checked() {
        let choices = [choice("zsh", "zsh"), choice("fish", "fish")];
        let menu = shell_menu(&choices, "fish", &WeakEntity::new_invalid());
        assert_eq!(menu.outline(), ["# New terminal with", "- zsh", "- fish [x]"]);
    }
}
