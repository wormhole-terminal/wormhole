//! The kitty keyboard protocol's flags: a stack per screen, pushed and popped by the program.
//!
//! Spec: <https://sw.kovidgoyal.net/kitty/keyboard-protocol/> ("Progressive enhancement"): `CSI > flags u`
//! pushes, `CSI < n u` pops n, `CSI = flags ; mode u` sets the top, `CSI ? u` asks. The main and
//! the alternate screen each have a stack, so a program that quits without popping can't leave
//! its flags on the other screen. Only the top matters to the host; the terminal mirrors it into
//! [`Modes`](super::Modes).

/// The most entries a stack keeps; pushing onto a full one drops the oldest (the protocol says
/// an implementation must bound it and that the oldest goes).
const DEPTH: usize = 32;

/// All five flags: disambiguate (1), event types (2), alternate keys (4), all keys as escape
/// codes (8), associated text (16).
const KNOWN: u8 = 0b1_1111;

/// How `CSI = flags ; mode u` combines the flags with the top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SetMode {
    /// 1 (the default): replace.
    Replace,
    /// 2: set the given bits.
    Or,
    /// 3: clear the given bits.
    AndNot,
}

impl SetMode {
    pub(super) fn from_param(n: u16) -> Option<SetMode> {
        match n {
            1 => Some(SetMode::Replace),
            2 => Some(SetMode::Or),
            3 => Some(SetMode::AndNot),
            _ => None,
        }
    }
}

/// One screen's stack. There is always a base entry (0), which popping never removes.
#[derive(Clone, Debug)]
pub(super) struct KeyboardFlags {
    stack: Vec<u8>,
}

impl Default for KeyboardFlags {
    fn default() -> Self {
        KeyboardFlags { stack: vec![0] }
    }
}

impl KeyboardFlags {
    /// The flags in force.
    pub(super) fn current(&self) -> u8 {
        self.stack.last().copied().unwrap_or(0)
    }

    pub(super) fn push(&mut self, flags: u8) {
        if self.stack.len() >= DEPTH {
            self.stack.remove(0);
        }
        self.stack.push(flags & KNOWN);
    }

    pub(super) fn pop(&mut self, count: usize) {
        for _ in 0..count {
            if self.stack.len() > 1 {
                self.stack.pop();
            } else {
                self.stack[0] = 0;
                break;
            }
        }
    }

    pub(super) fn set(&mut self, flags: u8, mode: SetMode) {
        let flags = flags & KNOWN;
        if let Some(top) = self.stack.last_mut() {
            *top = match mode {
                SetMode::Replace => flags,
                SetMode::Or => *top | flags,
                SetMode::AndNot => *top & !flags,
            };
        }
    }

    /// RIS.
    pub(super) fn reset(&mut self) {
        self.stack.clear();
        self.stack.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyboardFlags, SetMode};

    #[test]
    fn push_pop_and_set() {
        let mut keys = KeyboardFlags::default();
        keys.push(1);
        keys.push(9);
        assert_eq!(keys.current(), 9);
        keys.set(8, SetMode::AndNot);
        assert_eq!(keys.current(), 1);
        keys.set(4, SetMode::Or);
        assert_eq!(keys.current(), 5);
        keys.pop(1);
        assert_eq!(keys.current(), 1);
        keys.pop(10);
        assert_eq!(keys.current(), 0);
    }

    #[test]
    fn the_stack_is_bounded() {
        let mut keys = KeyboardFlags::default();
        for i in 0..100u8 {
            keys.push(i & 31);
        }
        assert_eq!(keys.stack.len(), super::DEPTH);
        assert_eq!(keys.current(), 99 & 31);
    }
}
