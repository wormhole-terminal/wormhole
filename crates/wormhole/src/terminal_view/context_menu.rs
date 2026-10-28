//! The menu a right-click on the terminal opens: the links and command blocks under the pointer,
//! Copy, Paste, Select All, Find, the splits and Clear. It is the native menu the rest of the app
//! uses (`platform::show_menu`). Titles and order follow the Edit menu where they are the same, and
//! each item shows its action's key from the keymap (the menu only shows it; the key is bound
//! elsewhere).
//!
//! The terminal reaches the window only by dispatching the actions the menu bar sends, so a split
//! is offered where the window would take one and nowhere else. The actions are the app's, a layer
//! above the terminal, so they are named by their registered names (a test in `actions` keeps the
//! names true).

use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::view::notice::Copied;
use wormhole_term::input::mouse;
use wormhole_text::{keys, text};

use super::TerminalView;
use super::blocks::BlockAction;
use super::input::TerminalCommand;
use super::links::open_uri;
use crate::menubar::model::KeyEquivalent;
use crate::platform::{self, Icon, Item, Menu};

/// The registered names of the window's split actions (`actions::SplitRight`, `SplitDown`).
pub(crate) const SPLIT_RIGHT: &str = "workspace::SplitRight";
pub(crate) const SPLIT_DOWN: &str = "workspace::SplitDown";

/// Whether the window handles the action called `name` from where the keyboard is: its split
/// actions are bound only while there is room for one.
fn action_available(name: &str, window: &Window, cx: &App) -> bool {
    window.available_actions(cx).iter().any(|action| action.name() == name)
}

/// Send the action called `name` to the window, as the menu bar does.
fn dispatch(name: &str, window: &mut Window, cx: &mut App) {
    match cx.build_action(name, None) {
        Ok(action) => window.dispatch_action(action, cx),
        Err(error) => tracing::warn!(name, %error, "the context menu's action isn't registered"),
    }
}

/// Whether a press of `button` opens the menu instead of reaching the program or the selection:
/// a right-click, or ⌃-click (macOS reads it as one, GPUI reports it as the left button). A program
/// that takes the mouse keeps both; ⇧ takes the click back from it, as it does for selecting.
pub(super) fn opens_menu(button: MouseButton, modifiers: Modifiers, program_takes_mouse: bool) -> bool {
    let secondary = match button {
        MouseButton::Right => true,
        MouseButton::Left => modifiers.control,
        _ => false,
    };
    secondary && (!program_takes_mouse || modifiers.shift)
}

/// What the menu is made from, as it is when it opens.
#[derive(Default)]
pub(super) struct MenuFacts {
    /// The address of the link under the pointer.
    pub(super) link: Option<String>,
    /// There is a selection to copy.
    pub(super) can_copy: bool,
    /// The clipboard holds text.
    pub(super) can_paste: bool,
    /// The finished command under the pointer.
    pub(super) block: Option<BlockFacts>,
    /// The window would take a split to the right / below from this terminal.
    pub(super) split_right: bool,
    pub(super) split_down: bool,
}

/// What the menu needs to know about the command block under the pointer.
pub(super) struct BlockFacts {
    pub(super) id: u64,
    pub(super) has_command: bool,
    pub(super) has_output: bool,
    /// The shell waits at its prompt, which is when a command can be typed there.
    pub(super) at_prompt: bool,
}

fn key(id: ActionId) -> Option<KeyEquivalent> {
    keymap::key(id).and_then(KeyEquivalent::from_spec)
}

fn symbol(name: &'static str) -> Option<Icon> {
    Some(Icon::Symbol(name))
}

/// The menu for `facts`; every item acts on `this`.
pub(super) fn terminal_menu(facts: MenuFacts, this: &WeakEntity<TerminalView>) -> Menu {
    let MenuFacts { link, can_copy, can_paste, block, split_right, split_down } = facts;
    let command = |title, icon, id, enabled, command: TerminalCommand| {
        Item::new(title).icon(symbol(icon)).key(key(id)).enabled(enabled).on_select(platform::run_on(
            this,
            move |terminal, _, cx| {
                terminal.run_command(command, cx);
                cx.notify();
            },
        ))
    };
    // The block actions have no key of their own, but a person may give them one.
    let block_item = |title, icon, key_id, id: u64, enabled, action: BlockAction| {
        Item::new(title)
            .icon(symbol(icon))
            .key(key(key_id))
            .enabled(enabled)
            .on_select(platform::run_on(this, move |terminal, _, cx| {
                terminal.block_action_on(id, action, cx)
            }))
    };
    let split = |title, icon, id, enabled, dispatch: fn(&mut Window, &mut App)| {
        Item::new(title).icon(symbol(icon)).key(key(id)).enabled(enabled).on_select(dispatch)
    };

    let mut menu = Menu::new();
    if let Some(uri) = link {
        let copied = uri.clone();
        menu = menu
            .item(
                Item::new(text(keys::TERMINAL_MENU_OPEN_LINK))
                    .icon(symbol("safari"))
                    .on_select(move |_, cx| open_uri(&uri, cx)),
            )
            .item(Item::new(text(keys::TERMINAL_MENU_COPY_LINK)).icon(symbol("link")).on_select(
                platform::run_on(this, move |terminal, _, cx| {
                    let notice = Copied::text(&copied);
                    platform::clipboard::write(copied.clone(), cx);
                    terminal.say(notice, cx);
                }),
            ));
    }
    menu = menu
        .separator()
        .item(command(
            text(keys::TERMINAL_MENU_COPY),
            "doc.on.doc",
            ActionId::Copy,
            can_copy,
            TerminalCommand::Copy,
        ))
        .item(command(
            text(keys::TERMINAL_MENU_PASTE),
            "doc.on.clipboard",
            ActionId::Paste,
            can_paste,
            TerminalCommand::Paste,
        ))
        .item(command(
            text(keys::TERMINAL_MENU_SELECT_ALL),
            "selection.pin.in.out",
            ActionId::SelectAll,
            true,
            TerminalCommand::SelectAll,
        ))
        .separator()
        .item(
            Item::new(text(keys::TERMINAL_MENU_FIND))
                .icon(symbol("magnifyingglass"))
                .key(key(ActionId::Find))
                .on_select(platform::run_on(this, |terminal, window, cx| {
                    let seed = terminal.find_seed();
                    terminal.open_find(seed, window, cx);
                })),
        );
    if let Some(BlockFacts { id, has_command, has_output, at_prompt }) = block {
        menu = menu
            .separator()
            .item(block_item(
                text(keys::TERMINAL_MENU_COPY_COMMAND),
                "terminal",
                ActionId::CopyCommand,
                id,
                has_command,
                BlockAction::CopyCommand,
            ))
            .item(block_item(
                text(keys::TERMINAL_MENU_COPY_OUTPUT),
                "text.alignleft",
                ActionId::CopyOutput,
                id,
                has_output,
                BlockAction::CopyOutput,
            ))
            .item(block_item(
                text(keys::TERMINAL_MENU_PUT_COMMAND_AT_PROMPT),
                "arrow.turn.down.left",
                ActionId::RerunCommand,
                id,
                has_command && at_prompt,
                BlockAction::PutAtPrompt,
            ));
    }
    menu.separator()
        .item(split(
            text(keys::TERMINAL_MENU_SPLIT_RIGHT),
            "rectangle.split.2x1",
            ActionId::SplitRight,
            split_right,
            |window, cx| dispatch(SPLIT_RIGHT, window, cx),
        ))
        .item(split(
            text(keys::TERMINAL_MENU_SPLIT_DOWN),
            "rectangle.split.1x2",
            ActionId::SplitDown,
            split_down,
            |window, cx| dispatch(SPLIT_DOWN, window, cx),
        ))
        .separator()
        .item(command(
            text(keys::TERMINAL_MENU_CLEAR),
            "eraser",
            ActionId::ClearBuffer,
            true,
            TerminalCommand::Clear,
        ))
}

impl TerminalView {
    /// A program is reading the mouse (clicks, or the DEC locator): its clicks are not ours.
    pub(super) fn program_takes_mouse(&self) -> bool {
        self.pty.is_some() && (mouse::wants_mouse(self.mode) || mouse::wants_locator(self.mode))
    }

    /// Open the context menu for a click at `position`. The selection stays as it is.
    pub(super) fn open_context_menu(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let block = self.block_under(position).map(|block| BlockFacts {
            id: block.id,
            has_command: !block.command.is_empty(),
            has_output: block.output_lines().is_some(),
            at_prompt: self.pty.as_ref().and_then(|pty| pty.at_prompt()) == Some(true),
        });
        let facts = MenuFacts {
            link: self.link_uri_under(position),
            can_copy: self.selection_text().is_some(),
            can_paste: platform::clipboard::read(cx).is_some_and(|text| !text.is_empty()),
            block,
            split_right: action_available(SPLIT_RIGHT, window, cx),
            split_down: action_available(SPLIT_DOWN, window, cx),
        };
        platform::show_menu(terminal_menu(facts, &cx.weak_entity()), position, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Modifiers, MouseButton, WeakEntity};

    use super::{BlockFacts, MenuFacts, opens_menu, terminal_menu};

    fn outline(facts: MenuFacts) -> Vec<String> {
        terminal_menu(facts, &WeakEntity::new_invalid()).outline()
    }

    fn shift() -> Modifiers {
        Modifiers { shift: true, ..Modifiers::default() }
    }

    fn control() -> Modifiers {
        Modifiers { control: true, ..Modifiers::default() }
    }

    #[test]
    fn copy_waits_for_a_selection_and_paste_for_text() {
        let bare = outline(MenuFacts::default());
        assert!(bare.contains(&"- Copy [off]".to_string()), "{bare:?}");
        assert!(bare.contains(&"- Paste [off]".to_string()), "{bare:?}");
        assert!(bare.contains(&"- Select All".to_string()), "{bare:?}");
        let ready = outline(MenuFacts { can_copy: true, can_paste: true, ..MenuFacts::default() });
        assert!(ready.contains(&"- Copy".to_string()) && ready.contains(&"- Paste".to_string()));
    }

    #[test]
    fn link_items_come_first_and_only_over_a_link() {
        let none = outline(MenuFacts::default());
        assert!(!none.iter().any(|row| row.contains("Link")), "{none:?}");
        let over = outline(MenuFacts { link: Some("https://example.com".into()), ..MenuFacts::default() });
        assert_eq!(over[..3], ["- Open Link", "- Copy Link", "---"]);
    }

    #[test]
    fn command_items_only_over_a_block_and_greyed_by_what_it_has() {
        assert!(!outline(MenuFacts::default()).iter().any(|row| row.contains("Command")));
        let block = BlockFacts { id: 7, has_command: true, has_output: false, at_prompt: false };
        let rows = outline(MenuFacts { block: Some(block), ..MenuFacts::default() });
        for expected in ["- Copy Command", "- Copy Output [off]", "- Put Command at Prompt [off]"] {
            assert!(rows.contains(&expected.to_string()), "{expected} in {rows:?}");
        }
        let block = BlockFacts { id: 7, has_command: true, has_output: true, at_prompt: true };
        let rows = outline(MenuFacts { block: Some(block), ..MenuFacts::default() });
        assert!(rows.contains(&"- Put Command at Prompt".to_string()));
    }

    #[test]
    fn splits_follow_the_room_and_clear_comes_last() {
        let rows = outline(MenuFacts { split_right: true, ..MenuFacts::default() });
        assert!(rows.contains(&"- Split Right".to_string()));
        assert!(rows.contains(&"- Split Down [off]".to_string()));
        assert_eq!(rows[rows.len() - 2..], ["---", "- Clear Buffer"]);
    }

    #[test]
    fn right_and_control_click_open_it_unless_a_program_has_the_mouse() {
        let plain = Modifiers::default();
        assert!(opens_menu(MouseButton::Right, plain, false));
        assert!(opens_menu(MouseButton::Left, control(), false));
        assert!(!opens_menu(MouseButton::Left, plain, false));
        assert!(!opens_menu(MouseButton::Middle, plain, false));
        // A program tracking the mouse keeps its clicks; ⇧ takes them back.
        assert!(!opens_menu(MouseButton::Right, plain, true));
        assert!(!opens_menu(MouseButton::Left, control(), true));
        assert!(opens_menu(MouseButton::Right, shift(), true));
    }
}
