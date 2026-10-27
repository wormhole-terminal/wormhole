//! The path's hover card: resting on the folder in the title row for a moment opens a native
//! popover under it with the whole path and three buttons: Open in Finder, Copy path (which says
//! "Copied" for a moment), and Insert into terminal, which types the path, quoted for the shell, at
//! the focused terminal's prompt as a paste (so an agent's input takes it like text you pasted).
//!
//! The popover stays open while the pointer is on it or on the path, closes a moment after it has
//! left both, and closes on Esc. A click on the path itself opens the folder in Finder, and a
//! right-click its Finder-style path menu (`path_menu`).

use std::path::Path;

use gpui_kit::*;
use wormhole_core::platform::shell;

use crate::platform::popover::{Behavior, Button, Glyph, Handle, Popover, Row};
use crate::platform::{self};
use crate::settings::store_in;
use crate::style::motion::{PATH_CARD_CLOSE_DELAY, PATH_CARD_COPIED_FOR, PATH_CARD_OPEN_DELAY};
use crate::terminal_view::Status;
use crate::window::Workspace;

/// What the pointer is over: the folder in the title row, or the card under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::window) enum Over {
    Path,
    Card,
}

#[derive(Default)]
pub(in crate::window) struct PathCard {
    /// The popover is up (or being opened).
    shown: bool,
    over_path: bool,
    over_card: bool,
    /// The timer that opens the popover, or closes it once the pointer has gone.
    timer: Option<Task<()>>,
    /// The open popover. Which one it is: a close that comes after another was opened is not
    /// about the new one.
    open: Option<(u64, Handle)>,
    opened: u64,
}

impl PathCard {
    /// Whether the card is on screen (devctl's `title`, the tests and the check on the chrome).
    #[cfg(any(test, debug_assertions, feature = "devctl"))]
    pub(in crate::window) fn is_shown(&self) -> bool {
        self.shown
    }

    /// Whether a popover is up: it is a window of its own, and the title row's window is no longer
    /// the key one for it.
    pub(in crate::window) fn is_open_popover(&self) -> bool {
        self.open.is_some()
    }

    /// Back to nothing, the popover closed: the window lost the front, the terminal changed under
    /// the pointer, or the folder was clicked.
    pub(in crate::window) fn reset(&mut self, cx: &mut App) {
        if let Some((_, handle)) = &self.open {
            handle.close(cx);
        }
        let opened = self.opened;
        *self = Self { opened, ..Self::default() };
    }
}

impl Workspace {
    /// The pointer came onto or left the folder or the card: open after a moment on either, close a
    /// moment after both are left (so crossing the gap between them doesn't close it).
    pub(in crate::window) fn hover_path(
        &mut self,
        over: Over,
        hovered: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let card = &mut self.path_card;
        match over {
            Over::Path => card.over_path = hovered,
            Over::Card => card.over_card = hovered,
        }
        let want = card.over_path || card.over_card;
        if want == card.shown {
            // Nothing to change: a timer that was about to do the opposite is no longer wanted.
            card.timer = None;
            return;
        }
        let delay = if want { PATH_CARD_OPEN_DELAY } else { PATH_CARD_CLOSE_DELAY };
        card.timer = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            this.update_in(cx, |this, window, cx| {
                let want = this.path_card.over_path || this.path_card.over_card;
                if want && !this.path_card.shown {
                    this.open_path_card(window, cx);
                } else if !want && this.path_card.shown {
                    this.path_card.reset(cx);
                    cx.notify();
                }
            })
            .ok();
        }));
    }

    /// Esc: close the card if it is open. Whether there was one.
    pub(in crate::window) fn close_path_card(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.path_card.shown {
            return false;
        }
        self.path_card.reset(cx);
        cx.notify();
        true
    }

    /// Open the card under the folder (the timer's, devctl's).
    pub(in crate::window) fn open_path_card(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let terminal = self.active_terminal().read(cx);
        let Some(folder) = terminal.cwd.clone() else { return };
        let exited = terminal.status() == Status::Exited;
        let shown = wormhole_core::view::sidebar::display_path(&folder, crate::style::home_dir());
        let appearance = store_in(&self.profile, cx).tokens.appearance;
        self.path_card.opened += 1;
        let which = self.path_card.opened;
        let popover = path_popover(&folder, &shown, exited, appearance, &cx.weak_entity(), which);
        let handle = platform::show_popover(popover, self.path_anchor.bounds(), window, cx);
        self.path_card.shown = true;
        self.path_card.open = Some((which, handle));
        cx.notify();
    }

    /// Copy path: the plain absolute path, unquoted, on the clipboard; VoiceOver says so.
    pub(in crate::window) fn copy_path(folder: &Path, cx: &mut App) {
        crate::platform::clipboard::write(folder.to_string_lossy().into_owned(), cx);
        crate::platform::a11y::announce("Path copied", false);
    }

    /// Insert into terminal: the folder's absolute path, in single quotes, typed at the focused
    /// terminal as a paste (bracketed when the program asked for it), without a Return. The
    /// popover closes and the terminal has the keyboard.
    pub(in crate::window) fn insert_path(
        &mut self,
        folder: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = insert_text(folder);
        let terminal = self.active_terminal();
        terminal.update(cx, |terminal, _| terminal.paste(&text));
        self.path_card.reset(cx);
        self.focus_terminal(window, cx);
        cx.notify();
    }
}

/// The card of `folder`, which reads `shown`; `which` is this opening of it.
fn path_popover(
    folder: &Path,
    shown: &str,
    exited: bool,
    appearance: wormhole_core::theme::tokens::Appearance,
    this: &WeakEntity<Workspace>,
    which: u64,
) -> Popover {
    let (open_of, copy_of, insert_of) = (folder.to_path_buf(), folder.to_path_buf(), folder.to_path_buf());
    let (over, closed) = (this.clone(), this.clone());
    Popover::new(Behavior::Hover, appearance)
        .row(Row::Path(shown.to_string().into()))
        .row(Row::Buttons(vec![
            Button::new("Open in Finder", Some(Glyph::Folder), move |_, cx| cx.open_with_system(&open_of)),
            Button::new("Copy path", Some(Glyph::Copy), move |_, cx| Workspace::copy_path(&copy_of, cx))
                .then_says("Copied", Some(Glyph::Check), PATH_CARD_COPIED_FOR),
            Button::new(
                "Insert into terminal",
                Some(Glyph::Terminal),
                platform::run_on(this, move |ws, window, cx| ws.insert_path(&insert_of, window, cx)),
            )
            .enabled(!exited, "No terminal is running"),
        ]))
        .on_hover(move |hovered, window, cx| {
            over.update(cx, |ws, cx| ws.hover_path(Over::Card, hovered, window, cx)).ok();
        })
        .on_close(move |_, cx| {
            closed
                .update(cx, |ws, cx| {
                    // Only the opening this is about.
                    if ws.path_card.open.as_ref().is_some_and(|(open, _)| *open == which) {
                        ws.path_card.open = None;
                        ws.path_card.shown = false;
                        cx.notify();
                    }
                })
                .ok();
        })
}

/// What Insert into terminal types: the path in single quotes, so spaces and `$` survive.
pub(in crate::window) fn insert_text(folder: &Path) -> String {
    shell::single_quote(&folder.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gpui_kit::WeakEntity;
    use wormhole_core::theme::tokens::Appearance;

    use super::{insert_text, path_popover};

    #[test]
    fn insert_types_the_absolute_path_in_single_quotes() {
        assert_eq!(insert_text(Path::new("/Users/ben/Projects/tube")), "'/Users/ben/Projects/tube'");
        assert_eq!(insert_text(Path::new("/Users/ben/My Projects/$HOME")), "'/Users/ben/My Projects/$HOME'");
        assert_eq!(insert_text(Path::new("/Users/ben/it's")), r#"'/Users/ben/it'\''s'"#);
    }

    #[test]
    fn the_card_has_the_path_and_three_buttons_and_copy_says_copied() {
        let popover = path_popover(
            Path::new("/Users/ben/Projects/tube"),
            "~/Projects/tube",
            false,
            Appearance::Dark,
            &WeakEntity::new_invalid(),
            1,
        );
        assert_eq!(
            popover.outline(),
            [
                "popover hover",
                "# ~/Projects/tube",
                "buttons: [Open in Finder] [Copy path -> Copied] [Insert into terminal]"
            ]
        );
        let exited =
            path_popover(Path::new("/tmp"), "/tmp", true, Appearance::Light, &WeakEntity::new_invalid(), 2);
        assert!(exited.outline()[2].contains("[Insert into terminal (off)]"));
    }
}
