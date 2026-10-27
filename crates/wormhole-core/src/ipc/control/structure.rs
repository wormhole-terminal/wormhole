//! Windows and splits from the command line: `new-window`, `split`, `focus`, `rename` and `close`,
//! what they ask (the specs), what they answer (the results) and the rules for what they carry.
//!
//! The switch (`control_write`) and the budgets are the same as for `new-terminal`, which these
//! verbs are kin to: they open terminals and windows, move the selection and end shells. None of
//! them types into a terminal or reads one, so none is refused for being the caller's own terminal:
//! an agent that names its own terminal, splits it for a dev server or asks to close it is doing
//! what the verbs are for. `close` is the exception in the other direction: it never closes
//! anything on a script's say. It asks the user, in the window's own close question.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{Error, ErrorCode, SEND_TEXT_MAX_BYTES, Target};

/// The longest name `rename` gives a terminal, in characters, the ellipsis of a cut one included.
pub const NAME_MAX_CHARS: usize = 64;

/// The most a command given to a new terminal or window may be: a prompt, not a file (as `send-text`).
pub const COMMAND_MAX_BYTES: usize = SEND_TEXT_MAX_BYTES;

/// `new-window`: a window with one new terminal, in `folder` (where the front window's current terminal
/// is without one), running `command` in its shell if given. It opens behind the other apps' windows and
/// takes no keys unless `activate`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewWindowSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub activate: bool,
}

/// Which side of the target's pane the new terminal goes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Right,
    Down,
    Left,
    Up,
}

impl Side {
    pub fn word(self) -> &'static str {
        match self {
            Side::Right => "right",
            Side::Down => "down",
            Side::Left => "left",
            Side::Up => "up",
        }
    }
}

/// `split`: a new terminal beside `target`, in its folder (or `folder`) and with its shell profile, joined
/// into the target's split (made if it stood alone). `size` is the new pane's share of the split, in
/// percent, held by the room as a divider's drag is. The selection and the keys stay where they are
/// unless `select`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitSpec {
    pub target: Target,
    #[serde(default)]
    pub direction: Side,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub select: bool,
}

/// `focus`: select the target's row, which shows its split. The keyboard stays where it was; `activate` gives
/// it to the terminal and brings the window forward, and the app with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusSpec {
    pub target: Target,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub activate: bool,
}

/// `rename`: the sidebar's rename. No name, or an empty one, calls the terminal after what runs in it again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameSpec {
    pub target: Target,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `close`: ask the user whether to close the target. Nothing closes on the caller's say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseSpec {
    pub target: Target,
}

/// `new-window`'s result: the ids a script chains on, known before the window is on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowOpened {
    pub window: i64,
    pub terminal: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

/// `split`'s result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitMade {
    pub window: i64,
    pub split: i64,
    /// The new terminal.
    pub terminal: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// The share of the split the new pane has, in whole percent, when `--size` was given and could be
    /// applied: the room may have held it short of what was asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u8>,
}

/// `focus`' result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Focused {
    pub window: i64,
    pub terminal: i64,
}

/// `rename`'s result: the name the sidebar shows now, and whether it is one given (not what runs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Renamed {
    pub terminal: i64,
    pub name: String,
    pub custom: bool,
}

/// `close`'s result: the question is up in `window`; `terminal_closed` follows if the user says yes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseAsked {
    pub asked: bool,
    pub window: i64,
    pub terminal: i64,
}

/// Whether the folder and the command a new terminal or window is given are fit: a folder that is
/// absolute and exists, a command on one line within [`COMMAND_MAX_BYTES`]. (A command is typed into the
/// shell like `send-text`'s paste; the switch for that is the caller's, `control_write`.)
pub fn launch_check(folder: Option<&str>, command: Option<&str>) -> Result<(), Error> {
    if let Some(folder) = folder {
        let path = Path::new(folder);
        if !path.is_absolute() || !path.is_dir() {
            return Err(Error::new(ErrorCode::NotFound, format!("no folder at {folder}")));
        }
    }
    if let Some(command) = command {
        if command.contains(['\n', '\r']) {
            return Err(Error::new(ErrorCode::BadRequest, "--command takes one line"));
        }
        if command.len() > COMMAND_MAX_BYTES {
            return Err(Error::new(
                ErrorCode::TooLarge,
                format!("--command takes at most {COMMAND_MAX_BYTES} bytes"),
            ));
        }
    }
    Ok(())
}

/// A `split --size` in range: a share of the split, in whole percent.
pub fn size_check(size: Option<u8>) -> Result<(), Error> {
    match size {
        Some(0) | Some(100..) => Err(Error::new(
            ErrorCode::BadRequest,
            "--size is the new pane's share of the split, 1 to 99 percent",
        )),
        _ => Ok(()),
    }
}

/// The name `rename` gives, cleaned as a notification's words are (one line, no control or invisible
/// characters, cut to [`NAME_MAX_CHARS`] with an ellipsis): `None` to call the terminal after what runs in
/// it again, for no name or an empty one. A name that is nothing once cleaned is a mistake, not a way to
/// clear: it would silently undo a name.
pub fn name_check(name: Option<&str>) -> Result<Option<String>, Error> {
    let Some(name) = name.filter(|name| !name.trim().is_empty()) else { return Ok(None) };
    // `clean` keeps `max` characters and adds the ellipsis, so one fewer keeps the whole within the cap.
    let cleaned = wormhole_term::clean(name, NAME_MAX_CHARS - 1);
    if cleaned.is_empty() {
        return Err(Error::new(
            ErrorCode::BadRequest,
            "nothing is left of that name once control and invisible characters are taken out",
        ));
    }
    Ok(Some(cleaned))
}

#[cfg(test)]
mod tests {
    use super::{
        COMMAND_MAX_BYTES, ErrorCode, NAME_MAX_CHARS, Side, SplitSpec, launch_check, name_check, size_check,
    };
    use crate::ipc::control::Target;

    fn code<T>(result: Result<T, super::Error>) -> Option<ErrorCode> {
        result.err().map(|error| error.code)
    }

    #[test]
    fn a_folder_is_absolute_and_there_and_a_command_is_one_short_line() {
        let here = std::env::temp_dir();
        let here = here.to_string_lossy();
        assert!(launch_check(Some(&here), Some("npm run dev")).is_ok());
        assert!(launch_check(None, None).is_ok());
        assert_eq!(code(launch_check(Some("relative/dir"), None)), Some(ErrorCode::NotFound));
        assert_eq!(code(launch_check(Some("/definitely/not/here/at/all"), None)), Some(ErrorCode::NotFound));
        assert_eq!(code(launch_check(None, Some("a\nb"))), Some(ErrorCode::BadRequest));
        assert_eq!(code(launch_check(None, Some("a\rb"))), Some(ErrorCode::BadRequest));
        let long = "x".repeat(COMMAND_MAX_BYTES);
        assert!(launch_check(None, Some(&long)).is_ok(), "at the cap is fine");
        assert_eq!(code(launch_check(None, Some(&format!("{long}x")))), Some(ErrorCode::TooLarge));
    }

    #[test]
    fn a_size_is_a_whole_percent_between_one_and_ninety_nine() {
        assert!(size_check(None).is_ok());
        assert!(size_check(Some(1)).is_ok() && size_check(Some(99)).is_ok());
        assert_eq!(code(size_check(Some(0))), Some(ErrorCode::BadRequest));
        assert_eq!(code(size_check(Some(100))), Some(ErrorCode::BadRequest));
    }

    #[test]
    fn a_name_is_cleaned_like_a_notifications_words_and_empty_clears() {
        assert_eq!(name_check(Some("api")).expect("fine"), Some("api".into()));
        assert_eq!(
            name_check(Some("  my \u{202e} api\n server ")).expect("fine"),
            Some("my api server".into())
        );
        assert_eq!(name_check(None).expect("clears"), None);
        assert_eq!(name_check(Some("")).expect("clears"), None);
        assert_eq!(name_check(Some("   ")).expect("clears"), None);
        let cut = name_check(Some(&"n".repeat(100))).expect("fine").expect("a name");
        assert_eq!(cut.chars().count(), NAME_MAX_CHARS, "cut, with its ellipsis, to the cap");
        assert!(cut.ends_with('…'));
        // Nothing left once cleaned is a mistake, not a clear.
        assert_eq!(code(name_check(Some("\u{1b}\u{200b}\u{7}"))), Some(ErrorCode::BadRequest));
    }

    /// A split with only a target asks for the right, as ⌘D does; the side is spelled in words.
    #[test]
    fn a_split_goes_right_unless_told() {
        let spec: SplitSpec =
            serde_json::from_str(r#"{"target":{"kind":"terminal","id":3}}"#).expect("reads");
        assert_eq!(spec.direction, Side::Right);
        let spec: SplitSpec =
            serde_json::from_str(r#"{"target":{"kind":"terminal","id":3},"direction":"up","size":40}"#)
                .expect("reads");
        assert_eq!((spec.direction, spec.size, spec.target), (Side::Up, Some(40), Target::Terminal(3)));
        assert_eq!(Side::Down.word(), "down");
    }
}
