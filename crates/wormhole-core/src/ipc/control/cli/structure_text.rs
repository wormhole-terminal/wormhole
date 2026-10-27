//! The reply to `new-window`, `split`, `focus`, `rename` and `close` as a person reads it: a line each,
//! with the ids a script would chain on. Names and folders are terminal text, so they are escaped.

use std::path::Path;

use super::render::shorten;
use crate::ipc::control::printable::for_terminal;
use crate::ipc::control::{CloseAsked, Focused, Renamed, SplitMade, WindowOpened};

/// ` at <folder>`, or nothing without one.
fn at(folder: Option<&str>, home: Option<&Path>) -> String {
    folder.map(|folder| format!(" at {}", for_terminal(&shorten(folder, home)))).unwrap_or_default()
}

pub(super) fn window_opened(opened: &WindowOpened, home: Option<&Path>) -> String {
    format!(
        "opened window {} with terminal {}{}\n",
        opened.window,
        opened.terminal,
        at(opened.folder.as_deref(), home)
    )
}

pub(super) fn split_made(made: &SplitMade, home: Option<&Path>) -> String {
    format!(
        "opened terminal {} in split {} in window {}{}\n",
        made.terminal,
        made.split,
        made.window,
        at(made.folder.as_deref(), home)
    )
}

pub(super) fn focused(focused: &Focused) -> String {
    format!("focused terminal {} in window {}\n", focused.terminal, focused.window)
}

pub(super) fn renamed(renamed: &Renamed) -> String {
    let name = for_terminal(&renamed.name);
    if renamed.custom {
        format!("terminal {} is named \"{name}\"\n", renamed.terminal)
    } else {
        format!("terminal {} is named after what runs in it, \"{name}\"\n", renamed.terminal)
    }
}

pub(super) fn close_asked(asked: &CloseAsked) -> String {
    format!(
        "asked in window {} whether to close terminal {}; it closes only if the user says yes\n",
        asked.window, asked.terminal
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{close_asked, focused, renamed, split_made, window_opened};
    use crate::ipc::control::{CloseAsked, Focused, Renamed, SplitMade, WindowOpened};

    #[test]
    fn each_reply_is_a_line_with_the_ids_to_chain_on() {
        let home = Some(Path::new("/Users/me"));
        let opened = WindowOpened { window: 2, terminal: 10, folder: Some("/Users/me/src/api".into()) };
        assert_eq!(window_opened(&opened, home), "opened window 2 with terminal 10 at ~/src/api\n");
        let none = WindowOpened { folder: None, ..opened };
        assert_eq!(window_opened(&none, home), "opened window 2 with terminal 10\n");
        let made = SplitMade { window: 1, split: 12, terminal: 9, folder: Some("/tmp".into()) };
        assert_eq!(split_made(&made, home), "opened terminal 9 in split 12 in window 1 at /tmp\n");
        assert_eq!(focused(&Focused { window: 1, terminal: 3 }), "focused terminal 3 in window 1\n");
        assert_eq!(
            renamed(&Renamed { terminal: 3, name: "api".into(), custom: true }),
            "terminal 3 is named \"api\"\n"
        );
        assert_eq!(
            renamed(&Renamed { terminal: 3, name: "zsh".into(), custom: false }),
            "terminal 3 is named after what runs in it, \"zsh\"\n"
        );
        assert_eq!(
            close_asked(&CloseAsked { asked: true, window: 1, terminal: 3 }),
            "asked in window 1 whether to close terminal 3; it closes only if the user says yes\n"
        );
    }

    /// A name or a folder is terminal text: whatever it holds cannot act on the terminal it is printed in.
    #[test]
    fn names_and_folders_are_escaped() {
        let evil = "x\u{1b}[2Jy\u{202e}z";
        let text = renamed(&Renamed { terminal: 1, name: evil.into(), custom: true });
        assert!(!text.contains(['\u{1b}', '\u{202e}']), "{text:?}");
        let text =
            split_made(&SplitMade { window: 1, split: 2, terminal: 3, folder: Some(evil.into()) }, None);
        assert!(!text.contains(['\u{1b}', '\u{202e}']), "{text:?}");
    }
}
