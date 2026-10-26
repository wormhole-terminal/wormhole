//! Who may do what: the rules the app and the tests share for typing into a terminal, starting
//! a program and the words of a notification.

use super::{Error, ErrorCode, TerminalState};

/// The most text one `send-text` may type (a prompt, not a file).
pub const SEND_TEXT_MAX_BYTES: usize = 16 * 1024;

/// What the app knows of a `send-text` target when it decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendTarget {
    pub tab: i64,
    pub state: TerminalState,
}

/// Whether `text` may go into `target` for `from`, with typing `enabled` in Settings. Order:
/// the switch first (so a caller learns about the setting before anything about a terminal),
/// then the size, then the terminal: never the caller's own (an agent answering its own prompt
/// from a script), never one waiting for the user's answer (a permission prompt is exactly what
/// a hostile script would want to type `y` into), never a dead shell.
pub fn send_text_check(
    enabled: bool,
    text: &str,
    from: Option<i64>,
    target: SendTarget,
) -> Result<(), Error> {
    let refuse = |code, message: String| Err(Error { code, message });
    if !enabled {
        return Err(writes_off());
    }
    if text.len() > SEND_TEXT_MAX_BYTES {
        return refuse(
            ErrorCode::TooLarge,
            format!("send-text takes at most {} bytes, not {}", SEND_TEXT_MAX_BYTES, text.len()),
        );
    }
    if text.trim().is_empty() {
        return refuse(ErrorCode::BadRequest, "nothing to send".to_string());
    }
    if from == Some(target.tab) {
        return refuse(
            ErrorCode::Refused,
            format!("tab {} is the one asking: a terminal can't type into itself", target.tab),
        );
    }
    match target.state {
        TerminalState::NeedsYou => refuse(
            ErrorCode::Refused,
            format!(
                "tab {} is waiting for the user's answer; nothing is typed into it until they have",
                target.tab
            ),
        ),
        TerminalState::Exited => refuse(ErrorCode::Refused, format!("tab {}'s shell has ended", target.tab)),
        TerminalState::Idle | TerminalState::Working | TerminalState::Finished | TerminalState::Failed => {
            Ok(())
        }
    }
}

/// The answer to `send-text` and `new-terminal` while `control_write` is off.
pub fn writes_off() -> Error {
    Error {
        code: ErrorCode::NotAllowed,
        message: "typing into terminals and opening them from the command line is off: turn on \
                  Settings → Coding agents → Let the command line type and open terminals"
            .into(),
    }
}

/// `notify`'s words as the banner shows them: cleaned like a program's own notification (one
/// line, no control or invisible characters, cut with an ellipsis), or nothing when nothing is
/// left to say.
pub fn notice_text(title: &str, body: &str) -> Option<wormhole_term::Notification> {
    use wormhole_term::{BODY_MAX_CHARS, TITLE_MAX_CHARS, clean};
    let (title, body) = (clean(title, TITLE_MAX_CHARS), clean(body, BODY_MAX_CHARS));
    (!title.is_empty() || !body.is_empty()).then_some(wormhole_term::Notification { title, body })
}

#[cfg(test)]
mod tests {
    use super::{
        Error, ErrorCode, SEND_TEXT_MAX_BYTES, SendTarget, TerminalState, notice_text, send_text_check,
    };

    fn check(enabled: bool, text: &str, from: Option<i64>, state: TerminalState) -> Result<(), ErrorCode> {
        send_text_check(enabled, text, from, SendTarget { tab: 5, state }).map_err(|Error { code, .. }| code)
    }

    #[test]
    fn text_goes_only_where_the_rules_allow() {
        // The switch first: a caller learns about Settings before anything about the terminal.
        assert_eq!(check(false, "hi", None, TerminalState::Idle), Err(ErrorCode::NotAllowed));
        assert_eq!(check(false, "hi", Some(5), TerminalState::NeedsYou), Err(ErrorCode::NotAllowed));
        assert_eq!(check(true, "hi", None, TerminalState::Idle), Ok(()));
        assert_eq!(check(true, "hi", Some(2), TerminalState::Working), Ok(()), "another agent at work");
        assert_eq!(check(true, "hi", None, TerminalState::Finished), Ok(()));
        assert_eq!(check(true, "hi", None, TerminalState::Failed), Ok(()), "back at its prompt");
        // Never the caller's own tab.
        assert_eq!(check(true, "y", Some(5), TerminalState::Idle), Err(ErrorCode::Refused));
        // Never a terminal waiting for the user: that is the permission prompt.
        assert_eq!(check(true, "y", Some(2), TerminalState::NeedsYou), Err(ErrorCode::Refused));
        assert_eq!(check(true, "y", None, TerminalState::NeedsYou), Err(ErrorCode::Refused));
        assert_eq!(check(true, "ls", None, TerminalState::Exited), Err(ErrorCode::Refused));
        // Caps.
        assert_eq!(check(true, "", None, TerminalState::Idle), Err(ErrorCode::BadRequest));
        assert_eq!(check(true, "  \n", None, TerminalState::Idle), Err(ErrorCode::BadRequest));
        let huge = "x".repeat(SEND_TEXT_MAX_BYTES + 1);
        assert_eq!(check(true, &huge, None, TerminalState::Idle), Err(ErrorCode::TooLarge));
        assert_eq!(check(true, &huge[1..], None, TerminalState::Idle), Ok(()), "at the cap is fine");
    }

    #[test]
    fn a_notices_words_are_cleaned_like_a_programs() {
        let notice =
            notice_text(" Deploy\u{202e} ", "finished\x07 in  3\u{200b}s\n").expect("something to say");
        assert_eq!((notice.title.as_str(), notice.body.as_str()), ("Deploy", "finished in 3s"));
        assert!(notice_text("\x07", "  ").is_none(), "nothing left to say");
        let long = notice_text("", &"a".repeat(1000)).expect("body");
        assert!(long.body.ends_with('…') && long.body.chars().count() == wormhole_term::BODY_MAX_CHARS + 1);
    }
}
