//! The values a host describes a key event with: [`Key`], [`Mods`] and [`KeyEvent`].
//!
//! They name what happened on the keyboard, not what any toolkit calls it: a GUI maps its own key
//! names onto [`Key`] once, and the encoders turn the event into the bytes a program expects.

use bitflags::bitflags;

bitflags! {
    /// The modifier keys held during an event.
    ///
    /// The bit values are the kitty keyboard protocol's, so the modifier field of its sequences is
    /// `1 + bits`. The lock modifiers only appear in kitty sequences: the legacy encoding has no
    /// room for them.
    ///
    /// Whether Option on macOS acts as Alt ("Option as Meta") is the host's policy, because only
    /// the host knows whether the layout composed a character with it. A host that wants Option to
    /// be Alt sends [`Mods::ALT`] with no `text`; one that wants Option to compose characters
    /// sends the composed `text` and leaves [`Mods::ALT`] out.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Mods: u8 {
        /// Shift.
        const SHIFT     = 1;
        /// Alt, Option or Meta on keyboards that call it that.
        const ALT       = 1 << 1;
        /// Control.
        const CTRL      = 1 << 2;
        /// Super: Command on macOS, the Windows key elsewhere.
        const SUPER     = 1 << 3;
        /// Hyper, a modifier few keyboards have.
        const HYPER     = 1 << 4;
        /// Meta, distinct from Alt (X11's Meta; xterm's modifier parameter 8).
        const META      = 1 << 5;
        /// Caps Lock is on.
        const CAPS_LOCK = 1 << 6;
        /// Num Lock is on.
        const NUM_LOCK  = 1 << 7;
    }
}

impl Mods {
    /// The lock states, which never change what a key means, only what the kitty protocol reports.
    pub(crate) const LOCKS: Mods = Mods::CAPS_LOCK.union(Mods::NUM_LOCK);

    /// The modifiers without the lock states.
    pub(crate) const fn without_locks(self) -> Mods {
        self.difference(Mods::LOCKS)
    }
}

/// A key on the keyboard, named by what it is and not by the character it types.
///
/// The names follow the kitty keyboard protocol's table
/// (<https://sw.kovidgoyal.net/kitty/keyboard-protocol/#functional-key-definitions>).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Key {
    /// A key that types a character, as the *unshifted* character on the layout in use: `a` for
    /// both `a` and Shift+`a`, `1` for Shift+`1` (which types `!`). A space is `Char(' ')`.
    Char(char),
    /// Enter / Return.
    Enter,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Escape.
    Escape,
    /// Cursor up.
    Up,
    /// Cursor down.
    Down,
    /// Cursor left.
    Left,
    /// Cursor right.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Insert.
    Insert,
    /// Forward delete.
    Delete,
    /// A function key, `F(1)` to `F(35)`. Other numbers encode to nothing.
    F(u8),
    /// A key of the numeric keypad.
    Keypad(KeypadKey),
    /// Caps Lock.
    CapsLock,
    /// Scroll Lock.
    ScrollLock,
    /// Num Lock.
    NumLock,
    /// Print Screen.
    PrintScreen,
    /// Pause / Break.
    Pause,
    /// The context menu key.
    Menu,
    /// A media key.
    Media(MediaKey),
    /// A modifier key pressed or released on its own.
    Modifier(ModifierKey),
}

impl Key {
    /// A function key if `n` is 1 to 35.
    pub fn function(n: u8) -> Option<Key> {
        (1..=35).contains(&n).then_some(Key::F(n))
    }
}

/// The keys of the numeric keypad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeypadKey {
    /// A digit key, `0` to `9`; larger values encode to nothing.
    Digit(u8),
    /// Decimal point.
    Decimal,
    /// Divide.
    Divide,
    /// Multiply.
    Multiply,
    /// Subtract.
    Subtract,
    /// Add.
    Add,
    /// Enter.
    Enter,
    /// Equals.
    Equal,
    /// Separator (the comma on some layouts).
    Separator,
    /// Left, with Num Lock off.
    Left,
    /// Right, with Num Lock off.
    Right,
    /// Up, with Num Lock off.
    Up,
    /// Down, with Num Lock off.
    Down,
    /// Page up, with Num Lock off.
    PageUp,
    /// Page down, with Num Lock off.
    PageDown,
    /// Home, with Num Lock off.
    Home,
    /// End, with Num Lock off.
    End,
    /// Insert, with Num Lock off.
    Insert,
    /// Delete, with Num Lock off.
    Delete,
    /// The key in the middle of the cross (5 with Num Lock off).
    Begin,
}

/// Media and volume keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MediaKey {
    /// Play.
    Play,
    /// Pause.
    Pause,
    /// Play / pause.
    PlayPause,
    /// Reverse.
    Reverse,
    /// Stop.
    Stop,
    /// Fast forward.
    FastForward,
    /// Rewind.
    Rewind,
    /// Next track.
    TrackNext,
    /// Previous track.
    TrackPrevious,
    /// Record.
    Record,
    /// Volume down.
    LowerVolume,
    /// Volume up.
    RaiseVolume,
    /// Mute.
    MuteVolume,
}

/// Modifier keys, left and right apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ModifierKey {
    /// Left Shift.
    LeftShift,
    /// Left Control.
    LeftControl,
    /// Left Alt.
    LeftAlt,
    /// Left Super.
    LeftSuper,
    /// Left Hyper.
    LeftHyper,
    /// Left Meta.
    LeftMeta,
    /// Right Shift.
    RightShift,
    /// Right Control.
    RightControl,
    /// Right Alt.
    RightAlt,
    /// Right Super.
    RightSuper,
    /// Right Hyper.
    RightHyper,
    /// Right Meta.
    RightMeta,
    /// ISO Level 3 Shift (AltGr).
    IsoLevel3Shift,
    /// ISO Level 5 Shift.
    IsoLevel5Shift,
}

/// What happened to the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum KeyKind {
    /// The key went down.
    #[default]
    Press,
    /// The key is held and the system repeats it.
    Repeat,
    /// The key came up. Only the kitty protocol, and only when the program asked for event types,
    /// reports it.
    Release,
}

/// One key event, as the host saw it.
///
/// Build one with [`KeyEvent::new`] and the `with_*` methods; the struct grows with the protocols,
/// so it cannot be written as a literal outside this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct KeyEvent<'a> {
    /// The key.
    pub key: Key,
    /// The modifiers held.
    pub mods: Mods,
    /// Press, repeat or release.
    pub kind: KeyKind,
    /// The text the key typed with the active layout and modifiers (`A` for Shift+`a`, `@` for
    /// Option+`l` on a German layout), or `None` when it typed nothing (a dead key, a shortcut,
    /// or a host that has no layout information). Without it, a [`Key::Char`] types its own
    /// character, upper-cased under Shift.
    pub text: Option<&'a str>,
    /// The character the key types with Shift held, for the kitty protocol's alternate keys. When
    /// `None` and Shift is held, the upper-case form of the key is used.
    pub shifted: Option<char>,
    /// The character the same physical key types on the standard US layout, for the kitty
    /// protocol's alternate keys; `None` when it is the key itself.
    pub base_layout: Option<char>,
}

impl<'a> KeyEvent<'a> {
    /// A key press without modifiers or text.
    pub const fn new(key: Key) -> KeyEvent<'a> {
        KeyEvent {
            key,
            mods: Mods::empty(),
            kind: KeyKind::Press,
            text: None,
            shifted: None,
            base_layout: None,
        }
    }

    /// The same event with these modifiers.
    #[must_use]
    pub const fn with_mods(mut self, mods: Mods) -> Self {
        self.mods = mods;
        self
    }

    /// The same event as a press, repeat or release.
    #[must_use]
    pub const fn with_kind(mut self, kind: KeyKind) -> Self {
        self.kind = kind;
        self
    }

    /// The same event with the text the key typed.
    #[must_use]
    pub const fn with_text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }

    /// The same event with the character the key types under Shift.
    #[must_use]
    pub const fn with_shifted(mut self, shifted: char) -> Self {
        self.shifted = Some(shifted);
        self
    }

    /// The same event with the character the key types on the US layout.
    #[must_use]
    pub const fn with_base_layout(mut self, base: char) -> Self {
        self.base_layout = Some(base);
        self
    }
}
