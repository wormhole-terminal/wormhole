//! What the host is told has changed: [`Event::ModesChanged`] and [`Event::ColorsChanged`].
//!
//! Neither costs anything while a program prints, and `feed` is untouched. When the host takes
//! the events ([`Terminal::events`]) the terminal compares a few words (the watched modes, the
//! cursor's look) with what the last call saw, and the palette only when a color command touched
//! it since. A change queues one event, at the end of what the host takes, however many
//! sequences and feeds caused it; a mode set and reset again in between is no change. Taking them
//! last keeps the events a feed made in their order and makes cutting the input at other places
//! change nothing the host sees. The host reads the new state itself ([`Terminal::modes`],
//! [`Terminal::cursor`], [`Terminal::colors`]) and so needs no payload.

use crate::terminal::{CursorStyle, Event, Terminal};
use crate::types::{Colors, Modes};

/// The modes a host acts on: the ones a change of which it must hear about to keep its input,
/// its drawing and its pointer right. Not the modes that only change how the engine places
/// text.
const WATCHED: Modes = Modes::SHOW_CURSOR
    .union(Modes::ALT_SCREEN)
    .union(Modes::ALTERNATE_SCROLL)
    .union(Modes::BRACKETED_PASTE)
    .union(Modes::FOCUS_REPORT)
    .union(Modes::APP_CURSOR)
    .union(Modes::APP_KEYPAD)
    .union(Modes::BACKARROW_BS)
    .union(Modes::MOUSE_CLICK)
    .union(Modes::MOUSE_DRAG)
    .union(Modes::MOUSE_MOTION)
    .union(Modes::MOUSE_X10)
    .union(Modes::MOUSE_SGR)
    .union(Modes::MOUSE_UTF8)
    .union(Modes::MOUSE_URXVT)
    .union(Modes::MOUSE_PIXELS)
    .union(Modes::KEYS_DISAMBIGUATE)
    .union(Modes::KEYS_EVENT_TYPES)
    .union(Modes::KEYS_ALTERNATE_KEYS)
    .union(Modes::KEYS_ALL_AS_ESCAPES)
    .union(Modes::KEYS_ASSOCIATED_TEXT)
    .union(Modes::REVERSE_VIDEO)
    .union(Modes::SYNC_OUTPUT)
    .union(Modes::THEME_REPORT)
    .union(Modes::LOCATOR);

/// What the host last heard about.
pub(in crate::terminal) struct Told {
    modes: Modes,
    cursor: CursorStyle,
    colors: Colors,
}

impl Told {
    /// The state of a terminal that has just been made: nothing to announce yet.
    pub(in crate::terminal) fn new() -> Told {
        Told { modes: Modes::INITIAL & WATCHED, cursor: CursorStyle::default(), colors: Colors::default() }
    }
}

impl Terminal {
    /// Queue `ModesChanged` and `ColorsChanged` for what changed since the last time; called when
    /// the host takes the events.
    pub(in crate::terminal) fn announce_changes(&mut self) {
        let modes = self.modes() & WATCHED;
        if modes != self.told.modes || self.cursor_style != self.told.cursor {
            self.told.modes = modes;
            self.told.cursor = self.cursor_style;
            self.push_event(Event::ModesChanged);
        }
        // Only a color command can have changed them: the common feed skips the comparison.
        if std::mem::take(&mut self.palette.touched) && self.palette.set != self.told.colors {
            self.told.colors = self.palette.set;
            self.push_event(Event::ColorsChanged);
        }
    }
}
