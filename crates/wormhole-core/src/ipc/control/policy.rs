//! Who may do what: the rules the app and the tests share for typing into a terminal, reading what
//! it shows, starting a program and the words of a notification.

use super::keys::{
    KEY_SPEC_MAX_BYTES, KEYS_MAX_ITEMS, KEYS_TEXT_MAX_BYTES, KeyItem, Stroke, parse_items, text_problem,
};
use super::{Error, ErrorCode, MouseOp, TerminalState};

/// The most text one `send-text` may type (a prompt, not a file).
pub const SEND_TEXT_MAX_BYTES: usize = 16 * 1024;

/// What the app knows of a `send-text` target when it decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendTarget {
    pub tab: i64,
    pub state: TerminalState,
    /// The caller's process runs under this terminal's shell (its `Peer`), whatever `from` claims.
    pub caller_inside: bool,
}

/// Whether anything may be typed into `target` for `from`: never the caller's own (an agent
/// answering its own prompt from a script: by the tab it names, or by where its process runs, so
/// leaving the claim out doesn't help), never one waiting for the user's answer (a permission prompt
/// is exactly what a hostile script would want to type `y` into), never a dead shell.
fn typing_target_check(from: Option<i64>, target: SendTarget) -> Result<(), Error> {
    let refuse = |message: String| Err(Error::new(ErrorCode::Refused, message));
    if from == Some(target.tab) || target.caller_inside {
        return refuse(format!("tab {} is the one asking: a terminal can't type into itself", target.tab));
    }
    match target.state {
        TerminalState::NeedsYou => refuse(format!(
            "tab {} is waiting for the user's answer; nothing is typed into it until they have",
            target.tab
        )),
        TerminalState::Exited => refuse(format!("tab {}'s shell has ended", target.tab)),
        TerminalState::Idle
        | TerminalState::Working
        | TerminalState::Finished
        | TerminalState::Failed
        | TerminalState::Other => Ok(()),
    }
}

/// Whether `text` may go into `target` for `from`, with typing `enabled` in Settings. Order:
/// the switch first (so a caller learns about the setting before anything about a terminal),
/// then the size, then the terminal ([`typing_target_check`]).
pub fn send_text_check(
    enabled: bool,
    text: &str,
    from: Option<i64>,
    target: SendTarget,
) -> Result<(), Error> {
    let refuse = |code, message: String| Err(Error::new(code, message));
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
    typing_target_check(from, target)
}

/// Whether `items` may go into `target` for `from`, with typing `enabled`; the strokes to send, in
/// order. The switch first, then the request (how many items, how long, whether each is a key or
/// printable text), then the terminal, as `send-text` does.
pub fn send_keys_check(
    enabled: bool,
    items: &[KeyItem],
    from: Option<i64>,
    target: SendTarget,
) -> Result<Vec<Stroke>, Error> {
    let refuse = |code, message: String| Err(Error::new(code, message));
    if !enabled {
        return Err(writes_off());
    }
    if items.is_empty() {
        return refuse(ErrorCode::BadRequest, "nothing to send".into());
    }
    if items.len() > KEYS_MAX_ITEMS {
        return refuse(
            ErrorCode::TooLarge,
            format!("send-keys takes at most {KEYS_MAX_ITEMS} keys and texts, not {}", items.len()),
        );
    }
    let mut text_bytes = 0;
    for item in items {
        match item {
            KeyItem::Key { key } if key.len() > KEY_SPEC_MAX_BYTES => {
                return refuse(
                    ErrorCode::TooLarge,
                    format!("a key name is at most {KEY_SPEC_MAX_BYTES} bytes"),
                );
            }
            KeyItem::Key { .. } => {}
            KeyItem::Text { text, .. } => {
                text_bytes += text.len();
                if let Some(problem) = text_problem(text) {
                    return refuse(ErrorCode::BadRequest, problem.into());
                }
            }
        }
    }
    if text_bytes > KEYS_TEXT_MAX_BYTES {
        return refuse(
            ErrorCode::TooLarge,
            format!("send-keys takes at most {KEYS_TEXT_MAX_BYTES} bytes of text, not {text_bytes}"),
        );
    }
    let strokes = parse_items(items).map_err(|e| Error::new(ErrorCode::BadRequest, e.to_string()))?;
    typing_target_check(from, target)?;
    Ok(strokes)
}

/// Whether `op` may be sent to `target` for `from`, with typing `enabled`: the same switch and
/// terminal rules as keys, and the request itself in range.
pub fn mouse_check(enabled: bool, op: &MouseOp, from: Option<i64>, target: SendTarget) -> Result<(), Error> {
    if !enabled {
        return Err(writes_off());
    }
    if let Some(problem) = op.problem() {
        return Err(Error::new(ErrorCode::BadRequest, problem));
    }
    typing_target_check(from, target)
}

/// Whether the caller may read what the target shows: `enabled` is `control_read`, `self_read`
/// that the caller's process runs under the target's own shell (its `Peer`, never what `from`
/// claims: a claim would let anyone switch the setting off for themselves).
pub fn read_check(enabled: bool, self_read: bool) -> Result<(), Error> {
    if enabled || self_read { Ok(()) } else { Err(reads_off()) }
}

/// The answer to `read`, `blocks` and `events --output` while `control_read` is off.
pub fn reads_off() -> Error {
    Error::new(
        ErrorCode::NotAllowed,
        "reading terminals from the command line is off (a terminal can always read itself): turn on \
         Settings → Coding agents → Let the command line read terminals",
    )
}

/// The answer to `send-text` and `new-terminal` while `control_write` is off.
pub fn writes_off() -> Error {
    Error::new(
        ErrorCode::NotAllowed,
        "typing into terminals and opening them from the command line is off: turn on \
         Settings → Coding agents → Let the command line type and open terminals",
    )
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
        Error, ErrorCode, KEYS_MAX_ITEMS, KEYS_TEXT_MAX_BYTES, KeyItem, MouseOp, SEND_TEXT_MAX_BYTES,
        SendTarget, TerminalState, mouse_check, notice_text, read_check, send_keys_check, send_text_check,
    };
    use crate::ipc::control::mouse::{ButtonName, Pointer};

    fn check(enabled: bool, text: &str, from: Option<i64>, state: TerminalState) -> Result<(), ErrorCode> {
        send_text_check(enabled, text, from, SendTarget { tab: 5, state, caller_inside: false })
            .map_err(|Error { code, .. }| code)
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

    /// The fix: leaving `from` out, or naming another tab, no longer lets a process type into the
    /// terminal it runs in.
    #[test]
    fn a_caller_inside_the_target_is_refused_whatever_it_claims() {
        for from in [None, Some(2), Some(5)] {
            let target = SendTarget { tab: 5, state: TerminalState::Idle, caller_inside: true };
            let refused = send_text_check(true, "hi", from, target).expect_err("refused");
            assert_eq!(refused.code, ErrorCode::Refused, "from {from:?}");
        }
        let elsewhere = SendTarget { tab: 5, state: TerminalState::Idle, caller_inside: false };
        assert_eq!(send_text_check(true, "hi", None, elsewhere), Ok(()));
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

    fn target(state: TerminalState, caller_inside: bool) -> SendTarget {
        SendTarget { tab: 5, state, caller_inside }
    }

    fn key(name: &str) -> KeyItem {
        KeyItem::Key { key: name.into() }
    }

    fn text(text: &str) -> KeyItem {
        KeyItem::Text { text: text.into(), paste: false }
    }

    fn keys_code(
        enabled: bool,
        items: &[KeyItem],
        from: Option<i64>,
        target: SendTarget,
    ) -> Result<usize, ErrorCode> {
        send_keys_check(enabled, items, from, target).map(|strokes| strokes.len()).map_err(|e| e.code)
    }

    /// The table of who may read: the setting, or the terminal reading itself by where it runs.
    #[test]
    fn reading_needs_the_setting_unless_the_caller_runs_in_the_terminal() {
        assert!(read_check(true, false).is_ok());
        assert!(read_check(false, true).is_ok(), "a terminal reads itself without the setting");
        assert!(read_check(true, true).is_ok());
        let off = read_check(false, false).expect_err("off");
        assert_eq!(off.code, ErrorCode::NotAllowed);
        assert!(off.message.contains("Let the command line read terminals"), "{}", off.message);
    }

    #[test]
    fn keys_go_only_where_text_would_and_in_the_size_the_caps_allow() {
        let idle = target(TerminalState::Idle, false);
        let ok = [key("ctrl-c"), text("ls -la"), key("enter")];
        assert_eq!(keys_code(true, &ok, None, idle), Ok(3));
        // The switch first, before anything about the request or the terminal.
        assert_eq!(
            keys_code(false, &ok, Some(5), target(TerminalState::NeedsYou, true)),
            Err(ErrorCode::NotAllowed)
        );
        // The same terminal rules as send-text.
        assert_eq!(keys_code(true, &ok, Some(5), idle), Err(ErrorCode::Refused));
        assert_eq!(
            keys_code(true, &ok, None, target(TerminalState::Idle, true)),
            Err(ErrorCode::Refused),
            "from left out"
        );
        assert_eq!(
            keys_code(true, &ok, Some(2), target(TerminalState::NeedsYou, false)),
            Err(ErrorCode::Refused)
        );
        assert_eq!(keys_code(true, &ok, None, target(TerminalState::Exited, false)), Err(ErrorCode::Refused));
        assert_eq!(keys_code(true, &ok, Some(2), target(TerminalState::Working, false)), Ok(3));
        // The request.
        assert_eq!(keys_code(true, &[], None, idle), Err(ErrorCode::BadRequest));
        let many = vec![key("a"); KEYS_MAX_ITEMS + 1];
        assert_eq!(keys_code(true, &many, None, idle), Err(ErrorCode::TooLarge));
        assert_eq!(keys_code(true, &many[1..], None, idle), Ok(KEYS_MAX_ITEMS));
        let long = [text(&"x".repeat(KEYS_TEXT_MAX_BYTES + 1))];
        assert_eq!(keys_code(true, &long, None, idle), Err(ErrorCode::TooLarge));
        assert_eq!(keys_code(true, &[text(&"x".repeat(KEYS_TEXT_MAX_BYTES))], None, idle), Ok(1));
        assert_eq!(keys_code(true, &[key(&"a".repeat(33))], None, idle), Err(ErrorCode::TooLarge));
        assert_eq!(
            keys_code(true, &[key("cmd-q")], None, idle),
            Err(ErrorCode::BadRequest),
            "the app's own key"
        );
        assert_eq!(keys_code(true, &[key("nope")], None, idle), Err(ErrorCode::BadRequest));
        for control in ["a\nb", "\u{1b}[31m", "tab\t", "\u{7f}"] {
            assert_eq!(
                keys_code(true, &[text(control)], None, idle),
                Err(ErrorCode::BadRequest),
                "{control:?}"
            );
        }
        let hostile = send_keys_check(true, &[key("\u{1b}[2J")], None, idle).expect_err("not a key");
        assert!(!hostile.message.contains('\u{1b}'), "{}", hostile.message);
    }

    #[test]
    fn the_mouse_follows_the_same_rules() {
        let click = MouseOp::Click(Pointer {
            row: 1,
            col: 1,
            button: ButtonName::Left,
            count: 1,
            shift: false,
            alt: false,
            ctrl: false,
        });
        let code = |enabled, from, target| mouse_check(enabled, &click, from, target).map_err(|e| e.code);
        assert_eq!(code(true, None, target(TerminalState::Idle, false)), Ok(()));
        assert_eq!(code(false, None, target(TerminalState::Idle, false)), Err(ErrorCode::NotAllowed));
        assert_eq!(code(true, Some(5), target(TerminalState::Idle, false)), Err(ErrorCode::Refused));
        assert_eq!(code(true, None, target(TerminalState::Idle, true)), Err(ErrorCode::Refused));
        assert_eq!(code(true, None, target(TerminalState::NeedsYou, false)), Err(ErrorCode::Refused));
        let nothing = MouseOp::Click(Pointer {
            count: 0,
            ..Pointer {
                row: 0,
                col: 0,
                button: ButtonName::Left,
                count: 1,
                shift: false,
                alt: false,
                ctrl: false,
            }
        });
        assert_eq!(
            mouse_check(true, &nothing, None, target(TerminalState::Idle, false)).map_err(|e| e.code),
            Err(ErrorCode::BadRequest)
        );
    }
}
