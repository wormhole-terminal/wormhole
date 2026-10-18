//! The window title and the icon name: OSC 0, 1 and 2, the stack XTWINOPS 22 and 23 push them on,
//! the title modes (`CSI > Ps t` and `T`) and the reports XTWINOPS 20 and 21 make of them.
//!
//! Spec: xterm's ctlseqs. OSC 0 sets the icon name and the window title, OSC 1 the icon name,
//! OSC 2 the title; an empty string resets. XTWINOPS 22 saves the icon name, the title or both,
//! 23 brings them back; a save is one slot of the stack, and a restore takes the slot that
//! holds the part it asks for. The title modes: 0 and 2 say the strings that set the title and
//! the icon name are hexadecimal or UTF-8, 1 and 3 that the strings that report them should
//! be; `CSI > Ps T` switches the same features off. A terminal that is UTF-8 throughout reads the
//! UTF-8 forms as the plain ones.
//!
//! The reports (`OSC l title ST`, `OSC L icon ST`) would put text a program chose into the input
//! of whatever program is running, so they are answered only when the host allows the window
//! operations ([`WindowOps`](super::WindowOps)), and control characters are taken out of them.

use std::borrow::Cow;

use bitflags::bitflags;

use super::event::Event;
use super::oscs::fill;
use super::{TITLE_MAX, TITLE_STACK, Terminal};

impl TitleModes {
    /// Each mode with the parameter of `CSI > Ps t` that switches it on.
    pub(super) fn parts(self) -> [(bool, usize); 2] {
        [(self.contains(TitleModes::SET_HEX), 0), (self.contains(TitleModes::QUERY_HEX), 1)]
    }
}

bitflags! {
    /// The title modes a program switched on.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(super) struct TitleModes: u8 {
        /// 0: strings that set a title are hexadecimal.
        const SET_HEX = 1;
        /// 1: titles are reported in hexadecimal.
        const QUERY_HEX = 1 << 1;
    }
}

/// The window title and the icon name a program set (OSC 2 and 1; an empty one is none), how it
/// wants them read and reported (XTSMTITLE), and the stack XTWINOPS 22 and 23 push and pop.
#[derive(Debug, Default)]
pub(super) struct Titles {
    pub(super) window: Option<String>,
    /// The icon name, kept for the title stack and the reports.
    pub(super) icon: Option<String>,
    /// The title modes a program switched on.
    pub(super) modes: TitleModes,
    pub(super) stack: Vec<SavedTitles>,
}

/// One slot of the title stack: the parts a save asked for.
#[derive(Clone, Debug)]
pub(super) struct SavedTitles {
    pub(super) window: Option<Option<String>>,
    pub(super) icon: Option<Option<String>>,
}

/// Which of the two XTWINOPS 22 and 23 mean: `0` both, `1` the icon name, `2` the title.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Parts {
    window: bool,
    icon: bool,
}

impl Parts {
    fn of(param: u16) -> Option<Parts> {
        match param {
            0 => Some(Parts { window: true, icon: true }),
            1 => Some(Parts { window: false, icon: true }),
            2 => Some(Parts { window: true, icon: false }),
            _ => None,
        }
    }
}

/// `text` as the string a title mode says it is: hexadecimal digits for two bytes each, or the
/// text itself. `None` for hexadecimal that is not.
fn decoded(modes: TitleModes, text: &[u8]) -> Option<Cow<'_, [u8]>> {
    if !modes.contains(TitleModes::SET_HEX) {
        // The usual case, once per prompt: no copy.
        return Some(Cow::Borrowed(text));
    }
    if !text.len().is_multiple_of(2) {
        return None;
    }
    text.chunks(2)
        .map(|pair| {
            let hi = char::from(pair[0]).to_digit(16)?;
            let lo = char::from(pair[1]).to_digit(16)?;
            Some((hi * 16 + lo) as u8)
        })
        .collect::<Option<Vec<u8>>>()
        .map(Cow::Owned)
}

impl Terminal {
    /// OSC 0 (both), 1 (the icon name) and 2 (the title), with the text as it came.
    pub(super) fn set_titles(&mut self, window: bool, icon: bool, text: &[u8]) {
        let Some(text) = decoded(self.titles.modes, text) else { return };
        if window {
            if text.is_empty() {
                self.titles.window = None;
            } else {
                fill(self.titles.window.get_or_insert_with(String::new), &text, TITLE_MAX);
            }
            self.push_event(Event::Title);
        }
        if icon {
            if text.is_empty() {
                self.titles.icon = None;
            } else {
                fill(self.titles.icon.get_or_insert_with(String::new), &text, TITLE_MAX);
            }
            // OSC 0 is announced as the title; the icon name has an event of its own for OSC 1.
            if !window {
                self.push_event(Event::IconName(self.titles.icon.clone().unwrap_or_default()));
            }
        }
    }

    /// The icon name the program set, `None` if it never did or reset it.
    pub fn icon_name(&self) -> Option<&str> {
        self.titles.icon.as_deref()
    }

    /// XTWINOPS 22: save the icon name, the title or both in one slot.
    pub(super) fn push_title(&mut self, which: u16) {
        let Some(parts) = Parts::of(which) else { return };
        if self.titles.stack.len() >= TITLE_STACK {
            self.titles.stack.remove(0);
        }
        self.titles.stack.push(SavedTitles {
            window: parts.window.then(|| self.titles.window.clone()),
            icon: parts.icon.then(|| self.titles.icon.clone()),
        });
    }

    /// XTWINOPS 23: bring back the icon name, the title or both, each from the newest slot that
    /// holds it; the slots used are gone.
    pub(super) fn pop_title(&mut self, which: u16) {
        let Some(parts) = Parts::of(which) else { return };
        let newest = |has: &dyn Fn(&SavedTitles) -> bool| self.titles.stack.iter().rposition(has);
        let window_slot = parts.window.then(|| newest(&|slot| slot.window.is_some())).flatten();
        let icon_slot = parts.icon.then(|| newest(&|slot| slot.icon.is_some())).flatten();
        if let Some(at) = window_slot
            && let Some(title) = self.titles.stack[at].window.clone()
        {
            self.titles.window = title;
            self.push_event(Event::Title);
        }
        if let Some(at) = icon_slot
            && let Some(icon) = self.titles.stack[at].icon.clone()
            && icon != self.titles.icon
        {
            self.titles.icon = icon;
            self.push_event(Event::IconName(self.titles.icon.clone().unwrap_or_default()));
        }
        match (window_slot, icon_slot) {
            (Some(a), Some(b)) if a == b => drop(self.titles.stack.remove(a)),
            (Some(a), Some(b)) => {
                // The later slot first, so the earlier one's place does not move.
                drop(self.titles.stack.remove(a.max(b)));
                drop(self.titles.stack.remove(a.min(b)));
            }
            (Some(at), None) | (None, Some(at)) => drop(self.titles.stack.remove(at)),
            (None, None) => {}
        }
    }

    /// `CSI > Ps ; … t` and `T`: switch title mode features on or off.
    pub(super) fn set_title_modes(&mut self, params: &[u16], on: bool) {
        for &param in params {
            let flag = match param {
                0 => TitleModes::SET_HEX,
                1 => TitleModes::QUERY_HEX,
                // UTF-8 is what every string is; asking for it is asking for the plain form.
                2 | 3 => {
                    self.titles.modes.remove(if param == 2 {
                        TitleModes::SET_HEX
                    } else {
                        TitleModes::QUERY_HEX
                    });
                    continue;
                }
                _ => continue,
            };
            self.titles.modes.set(flag, on);
        }
    }

    /// XTWINOPS 20 (the icon name) and 21 (the title): `OSC L text ST` and `OSC l text ST`.
    pub(super) fn report_title(&mut self, icon: bool) {
        let text =
            if icon { self.titles.icon.clone() } else { self.titles.window.clone() }.unwrap_or_default();
        // Control characters, C1 included, are text a terminal must not hand a program back.
        let text: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
        let letter = if icon { 'L' } else { 'l' };
        if self.titles.modes.contains(TitleModes::QUERY_HEX) {
            self.reply(format_args!("\x1b]{letter}{}\x1b\\", super::dcs::Hex(text.as_bytes())));
        } else {
            self.reply(format_args!("\x1b]{letter}{text}\x1b\\"));
        }
    }
}
