//! Key specs like `cmd-shift-r`, `ctrl-alt-t`, `cmd--` (the minus key) or `cmd+shift+space`: one
//! grammar and one parser for project action keys and the global hotkey. Matching a key press is
//! left to each user (GPUI keystrokes, Carbon key codes); this only knows the text form.
//!
//! Grammar: modifiers first, the key last, separated by `-` or `+`. A separator character as the
//! key is written after a separator (`cmd--`, `ctrl++`). Modifier names: `cmd` (`command`,
//! `super`, `meta`, `platform`, `⌘`), `ctrl` (`control`, `⌃`), `alt` (`opt`, `option`, `⌥`),
//! `shift` (`⇧`); case doesn't matter.

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct KeyMods {
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// A parsed key spec. `key` is lowercase, with aliases resolved (`return` is `enter`, `esc` is
/// `escape`, `backtick` is `` ` ``).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeySpec {
    pub mods: KeyMods,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySpecError {
    Empty,
    /// The spec ends in modifiers (`cmd-`).
    MissingKey(String),
    UnknownModifier(String),
    DuplicateModifier(String),
}

impl fmt::Display for KeySpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty key"),
            Self::MissingKey(spec) => write!(f, "missing key in {spec:?}"),
            Self::UnknownModifier(token) => write!(f, "unknown modifier {token:?}"),
            Self::DuplicateModifier(token) => write!(f, "duplicate modifier {token:?}"),
        }
    }
}

impl std::error::Error for KeySpecError {}

/// Named keys besides single characters, as GPUI names them (`f1`…`f20` are added on top).
const NAMED_KEYS: &[&str] = &[
    "enter",
    "escape",
    "space",
    "tab",
    "backspace",
    "delete",
    "up",
    "down",
    "left",
    "right",
    "home",
    "end",
    "pageup",
    "pagedown",
];

/// Arrow and editing keys and the glyph Apple writes for each, both ways.
const KEY_GLYPHS: &[(&str, &str)] = &[
    ("enter", "↩"),
    ("tab", "⇥"),
    ("escape", "⎋"),
    ("backspace", "⌫"),
    ("delete", "⌦"),
    ("up", "↑"),
    ("down", "↓"),
    ("left", "←"),
    ("right", "→"),
];

/// The US layout's shifted punctuation and digits: what a keyboard reports for ⇧ and the key
/// (`⇧]` arrives as `}` without ⇧), so that both spellings name the same key.
const US_SHIFTED: &[(&str, &str)] = &[
    ("`", "~"),
    ("1", "!"),
    ("2", "@"),
    ("3", "#"),
    ("4", "$"),
    ("5", "%"),
    ("6", "^"),
    ("7", "&"),
    ("8", "*"),
    ("9", "("),
    ("0", ")"),
    ("-", "_"),
    ("=", "+"),
    ("[", "{"),
    ("]", "}"),
    ("\\", "|"),
    (";", ":"),
    ("'", "\""),
    (",", "<"),
    (".", ">"),
    ("/", "?"),
];

impl KeySpec {
    pub fn new(mods: KeyMods, key: &str) -> Self {
        Self { mods, key: key.to_lowercase() }
    }

    /// Whether `key` is a single character or a key [`NAMED_KEYS`] (or `f1`…`f20`) knows: a key
    /// name that is neither is a typo that would never match.
    pub fn has_known_key(&self) -> bool {
        self.key.chars().count() == 1 || NAMED_KEYS.contains(&self.key.as_str()) || self.is_function_key()
    }

    /// `f1`…`f20`, whatever the modifiers.
    pub fn is_function_key(&self) -> bool {
        self.key.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()).is_some_and(|n| (1..=20).contains(&n))
    }

    /// Whether a press of `key` (lowercase, GPUI's names) with exactly `mods` held is this key.
    /// A shifted character counts as ⇧ and its key (`}` is `⇧]` on a US keyboard).
    pub fn matches(&self, mods: KeyMods, key: &str) -> bool {
        self.same_key(&KeySpec { mods, key: key.to_string() })
    }

    /// Whether two specs name the same key press, written either way (`cmd-}` and `shift-cmd-]`).
    /// It runs for every row of the keymap on every keystroke, so it compares without copying.
    pub fn same_key(&self, other: &KeySpec) -> bool {
        self.canonical_parts() == other.canonical_parts()
    }

    /// The spec with a US layout's shifted character written as ⇧ and its key: `cmd-}` is
    /// `shift-cmd-]`, `cmd-+` is `shift-cmd-=`.
    pub fn canonical(&self) -> KeySpec {
        let (mods, key) = self.canonical_parts();
        KeySpec { mods, key: key.to_string() }
    }

    fn canonical_parts(&self) -> (KeyMods, &str) {
        match US_SHIFTED.iter().find(|(_, shifted)| *shifted == self.key) {
            Some((base, _)) => (KeyMods { shift: true, ..self.mods }, base),
            None => (self.mods, &self.key),
        }
    }

    /// The character a US keyboard reports for ⇧ and this key (`}` for `⇧]`), when that is not
    /// just the upper case letter: GPUI then reports the character without ⇧, so a binding needs
    /// that spelling too.
    pub fn shifted_spelling(&self) -> Option<KeySpec> {
        if !self.mods.shift {
            return None;
        }
        let (_, shifted) = US_SHIFTED.iter().find(|(base, _)| *base == self.key)?;
        Some(KeySpec { mods: KeyMods { shift: false, ..self.mods }, key: shifted.to_string() })
    }

    /// Parse Apple's notation, as [`KeySpec::glyphs`] writes it: modifier glyphs in any order,
    /// then the key (`⌥⌘1`, `⇧⌘]`, `⌃⌘↓`, `⇧F6`, `⌥Space`). `None` for anything else.
    pub fn from_glyphs(text: &str) -> Option<KeySpec> {
        let text = text.trim();
        let mut mods = KeyMods::default();
        let mut rest = text;
        while let Some(c) = rest.chars().next() {
            let flag = match c {
                '⌃' => &mut mods.ctrl,
                '⌥' => &mut mods.alt,
                '⇧' => &mut mods.shift,
                '⌘' => &mut mods.cmd,
                _ => break,
            };
            if *flag {
                return None;
            }
            *flag = true;
            rest = &rest[c.len_utf8()..];
        }
        if rest.is_empty() {
            return None;
        }
        if let Some((name, _)) = KEY_GLYPHS.iter().find(|(_, glyph)| *glyph == rest) {
            return Some(KeySpec { mods, key: name.to_string() });
        }
        let key = rest.to_lowercase();
        let spec = KeySpec { mods, key };
        (spec.key.chars().count() == 1 || NAMED_KEYS.contains(&spec.key.as_str()) || spec.is_function_key())
            .then_some(spec)
    }

    /// Apple's notation, modifiers in ⌃⌥⇧⌘ order: `⇧⌘R`, `⌥Space`, `⌘-`.
    pub fn glyphs(&self) -> String {
        let mut out = String::new();
        for (on, glyph) in
            [(self.mods.ctrl, '⌃'), (self.mods.alt, '⌥'), (self.mods.shift, '⇧'), (self.mods.cmd, '⌘')]
        {
            if on {
                out.push(glyph);
            }
        }
        let mut chars = self.key.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => out.extend(c.to_uppercase()),
            _ => match KEY_GLYPHS.iter().find(|(name, _)| *name == self.key) {
                Some((_, glyph)) => out.push_str(glyph),
                None => {
                    let mut name = self.key.chars();
                    out.extend(name.next().into_iter().flat_map(char::to_uppercase));
                    out.push_str(name.as_str());
                }
            },
        }
        out
    }
}

impl FromStr for KeySpec {
    type Err = KeySpecError;

    fn from_str(spec: &str) -> Result<Self, KeySpecError> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err(KeySpecError::Empty);
        }
        let is_sep = |c: char| c == '-' || c == '+';
        // The key itself may be a separator character ("cmd--", "cmd-+"): it's then the last char
        // and directly preceded by a separator.
        let (mods_part, key) = match spec.char_indices().last() {
            Some((i, last)) if is_sep(last) && i > 0 && spec[..i].ends_with(is_sep) => {
                (&spec[..i - 1], &spec[i..])
            }
            Some((i, last)) if is_sep(last) && i == 0 => ("", spec),
            _ => match spec.rfind(is_sep) {
                Some(i) => (&spec[..i], &spec[i + 1..]),
                None => ("", spec),
            },
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(KeySpecError::MissingKey(spec.to_string()));
        }
        let mut mods = KeyMods::default();
        if !mods_part.is_empty() {
            for token in mods_part.split(is_sep) {
                let token = token.trim();
                let flag = match token.to_lowercase().as_str() {
                    "cmd" | "command" | "super" | "meta" | "platform" | "⌘" => &mut mods.cmd,
                    "ctrl" | "control" | "⌃" => &mut mods.ctrl,
                    "alt" | "opt" | "option" | "⌥" => &mut mods.alt,
                    "shift" | "⇧" => &mut mods.shift,
                    _ => return Err(KeySpecError::UnknownModifier(token.to_string())),
                };
                if *flag {
                    return Err(KeySpecError::DuplicateModifier(token.to_string()));
                }
                *flag = true;
            }
        }
        let key = match key.to_lowercase().as_str() {
            "return" => "enter".to_string(),
            "esc" => "escape".to_string(),
            "backtick" | "grave" => "`".to_string(),
            other => other.to_string(),
        };
        Ok(Self { mods, key })
    }
}

/// The canonical text form, `ctrl-alt-shift-cmd-<key>`.
impl fmt::Display for KeySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (on, name) in [
            (self.mods.ctrl, "ctrl"),
            (self.mods.alt, "alt"),
            (self.mods.shift, "shift"),
            (self.mods.cmd, "cmd"),
        ] {
            if on {
                write!(f, "{name}-")?;
            }
        }
        f.write_str(&self.key)
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyMods, KeySpec, KeySpecError};

    fn spec(text: &str) -> KeySpec {
        text.parse().expect("valid")
    }

    fn mods(cmd: bool, ctrl: bool, alt: bool, shift: bool) -> KeyMods {
        KeyMods { cmd, ctrl, alt, shift }
    }

    #[test]
    fn parses_names_aliases_and_separators() {
        assert_eq!(spec("cmd-shift-r"), KeySpec { mods: mods(true, false, false, true), key: "r".into() });
        assert_eq!(spec(" Command+Shift+R "), spec("cmd-shift-r"));
        assert_eq!(spec("⌃-⌥-t"), spec("ctrl-alt-t"));
        assert_eq!(spec("platform-option-return").key, "enter");
        assert_eq!(spec("ctrl-backtick").key, "`");
        assert_eq!(spec("f12"), KeySpec { mods: KeyMods::default(), key: "f12".into() });
        assert_eq!(spec("alt-Space").key, "space");
    }

    #[test]
    fn a_separator_can_be_the_key() {
        assert_eq!(spec("cmd--"), KeySpec { mods: mods(true, false, false, false), key: "-".into() });
        assert_eq!(spec("ctrl+-").key, "-");
        assert_eq!(spec("ctrl-shift-=").key, "=");
        assert_eq!(spec("cmd-+").key, "+");
        assert_eq!(spec("-").key, "-");
    }

    #[test]
    fn rejects_malformed_specs() {
        assert_eq!("".parse::<KeySpec>(), Err(KeySpecError::Empty));
        assert_eq!("cmd-".parse::<KeySpec>(), Err(KeySpecError::MissingKey("cmd-".into())));
        assert_eq!("⌃⌥-t".parse::<KeySpec>(), Err(KeySpecError::UnknownModifier("⌃⌥".into())));
        assert_eq!("hyper-t".parse::<KeySpec>(), Err(KeySpecError::UnknownModifier("hyper".into())));
        assert_eq!("cmd-t-alt".parse::<KeySpec>(), Err(KeySpecError::UnknownModifier("t".into())));
        assert_eq!(
            "cmd-command-t".parse::<KeySpec>(),
            Err(KeySpecError::DuplicateModifier("command".into()))
        );
        assert_eq!(KeySpecError::UnknownModifier("x".into()).to_string(), "unknown modifier \"x\"");
    }

    #[test]
    fn known_keys() {
        for known in ["cmd-a", "cmd-1", "cmd--", "cmd-enter", "cmd-f20", "cmd-pagedown", "cmd-é"] {
            assert!(spec(known).has_known_key(), "{known}");
        }
        for unknown in ["cmd-nosuchkey", "cmd-f21", "cmd-f0", "cmd-fx"] {
            assert!(!spec(unknown).has_known_key(), "{unknown}");
        }
    }

    #[test]
    fn display_forms() {
        assert_eq!(spec("cmd-ctrl-alt-shift-t").glyphs(), "⌃⌥⇧⌘T");
        assert_eq!(spec("alt-space").glyphs(), "⌥Space");
        assert_eq!(spec("cmd--").glyphs(), "⌘-");
        assert_eq!(spec("cmd-enter").glyphs(), "⌘↩");
        assert_eq!(spec("f5").glyphs(), "F5");
        assert_eq!(spec("Command+Shift+R").to_string(), "shift-cmd-r");
        assert_eq!(spec(&spec("ctrl-alt--").to_string()), spec("ctrl-alt--"), "round trip");
    }

    #[test]
    fn matching_is_exact() {
        let s = spec("cmd-shift-r");
        assert!(s.matches(mods(true, false, false, true), "r"));
        assert!(!s.matches(mods(true, false, false, false), "r"));
        assert!(!s.matches(mods(true, true, false, true), "r"));
        assert!(!s.matches(mods(true, false, false, true), "t"));
    }

    /// A US keyboard reports ⇧⌘] as ⌘} (GPUI drops the ⇧ for anything but a letter), so a spec
    /// written the way people read it has to match that press, and the two spellings are one key.
    #[test]
    fn a_shifted_character_is_shift_and_its_key() {
        let next_tab = spec("shift-cmd-]");
        assert!(next_tab.matches(mods(true, false, false, false), "}"));
        assert!(next_tab.matches(mods(true, false, false, true), "]"));
        assert!(spec("cmd-}").same_key(&next_tab));
        assert!(spec("cmd-+").same_key(&spec("shift-cmd-=")));
        assert!(!spec("cmd-=").same_key(&spec("cmd-+")));
        assert!(!spec("cmd-]").same_key(&next_tab));
        assert_eq!(next_tab.shifted_spelling(), Some(spec("cmd-}")));
        assert_eq!(spec("shift-cmd-n").shifted_spelling(), None, "letters keep their ⇧");
        assert_eq!(spec("cmd-]").shifted_spelling(), None);
    }

    #[test]
    fn glyphs_round_trip() {
        for text in ["⌥⌘1", "⇧⌘]", "⌃⌘↓", "F6", "⇧F6", "⌥Space", "⌘-", "⌘+", "⌘,", "⌥⇧⌘I", "⌘⌫", "⌘?"]
        {
            let parsed = KeySpec::from_glyphs(text).unwrap_or_else(|| panic!("{text} parses"));
            assert_eq!(parsed.glyphs(), text, "{text}");
        }
        assert_eq!(KeySpec::from_glyphs("⌥⌘1"), Some(spec("alt-cmd-1")));
        assert_eq!(KeySpec::from_glyphs("⌃⌘↓"), Some(spec("ctrl-cmd-down")));
        assert_eq!(KeySpec::from_glyphs("⇧⌥⌘I"), Some(spec("shift-alt-cmd-i")), "any order in");
        assert_eq!(KeySpec::from_glyphs("⌘⌘T"), None, "a modifier twice");
        assert_eq!(KeySpec::from_glyphs("⌘"), None, "no key");
        assert_eq!(KeySpec::from_glyphs("⌘Nope"), None);
        assert!(spec("f20").is_function_key() && spec("shift-f6").is_function_key());
        assert!(!spec("f21").is_function_key() && !spec("cmd-f").is_function_key());
    }
}
