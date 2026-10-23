//! Opening a terminal with a shell other than the default: the menu under a right-click on the +
//! of the Terminals title and of the tab bar, as Terminal.app's "New Tab with Profile". The shells on offer, their
//! keys and what each runs are core's `shells`; the Shell menu's "New Terminal With ▸" and the
//! palette's "New terminal: fish" entries go through the same method. The menu is the same wherever
//! the + is; what choosing a shell does is up to whoever puts it there.

use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::terminal::shells::{self, ShellChoice};

use crate::platform::{self, Item, Menu};
use crate::settings::settings;

/// The menu's heading.
const MENU_TITLE: &str = "New terminal with";

/// What choosing a shell (by its key) does.
pub(crate) type OnShell = Rc<dyn Fn(String, &mut Window, &mut App)>;

/// `button` (a +) with the shells' menu under a right-click: every shell by name, the default
/// (what a left click opens) checked.
pub(crate) fn with_shell_menu(id: &'static str, button: impl IntoElement, on_shell: OnShell) -> AnyElement {
    div()
        .id(id)
        .relative()
        .flex_none()
        .debug_selector(move || id.to_string())
        .child(button)
        // Over the button, which blocks the pointer from what lies behind it.
        .child(platform::on_right_click(move |at, window, cx| {
            // The shells and the default as they are when the menu opens.
            let (choices, default) = {
                let settings = settings(cx);
                (shells::choices(settings), shells::default_key(settings).to_string())
            };
            platform::show_menu(shell_menu(&choices, &default, &on_shell), at, window, cx);
        }))
        .into_any_element()
}

/// The menu of `choices`, `default` checked; choosing one calls `on_shell` with its key.
fn shell_menu(choices: &[ShellChoice], default: &str, on_shell: &OnShell) -> Menu {
    choices.iter().fold(Menu::new().header(MENU_TITLE), |menu, choice| {
        let key = choice.key.clone();
        let on_shell = on_shell.clone();
        menu.item(
            Item::new(choice.name.clone())
                .checked(choice.key == default)
                .on_select(move |window, cx| on_shell(key.clone(), window, cx)),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::shell_menu;
    use wormhole_core::terminal::shells::{ShellChoice, ShellKind};

    fn no_shell(_: String, _: &mut gpui_kit::Window, _: &mut gpui_kit::App) {}

    fn choice(key: &str, name: &str) -> ShellChoice {
        ShellChoice { key: key.into(), name: name.into(), command_line: name.into(), kind: ShellKind::System }
    }

    /// The default shell is the checked one, under the heading.
    #[test]
    fn the_default_shell_is_checked() {
        let choices = [choice("zsh", "zsh"), choice("fish", "fish")];
        let nothing: super::OnShell = Rc::new(no_shell);
        let menu = shell_menu(&choices, "fish", &nothing);
        assert_eq!(menu.outline(), ["# New terminal with", "- zsh", "- fish [x]"]);
    }
}
