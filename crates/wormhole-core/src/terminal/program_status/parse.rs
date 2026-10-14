//! The grammar of OSC 7501 and its limits, applied to the body of one sequence.
//!
//! ```text
//! OSC 7501 ; pairs ST        report
//! OSC 7501 ; ? ST            feature detection
//! pairs := pair (":" pair)*      pair := key "=" value
//! key   := [a-z]+                value := [A-Za-z0-9_.,+/=-]*
//! ```
//!
//! A report is checked in full before anything is returned: a report that breaks a limit, whose
//! base64 does not decode or whose decoded text holds a control character is discarded whole
//! (`None`), and so is one whose `state` or `id` the terminal does not recognize. The rest is
//! forgiving by design, so a later revision can add keys: a malformed pair is skipped, an unknown
//! key ignored, a repeated key decided by its last pair, and a `kind`, `progress` or `app` that
//! doesn't fit is simply absent.

use base64::Engine;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};

use super::{Kind, Record, Report, State};

/// The whole sequence, from OSC to ST.
pub const SEQUENCE_MAX: usize = 4096;
/// `ESC ] 7501 ;`
const PREFIX_LEN: usize = 7;
const KEY_MAX: usize = 16;
const MSG_ENCODED_MAX: usize = 2732;
const MSG_DECODED_MAX: usize = 2048;
const TITLE_ENCODED_MAX: usize = 256;
const TITLE_DECODED_MAX: usize = 192;
const APP_MAX: usize = 32;
const ID_MAX: usize = 128;
const SEGMENT_MAX: usize = 32;
const ID_DEPTH_MAX: usize = 8;

/// Standard base64 with optional padding. Trailing bits are not checked: a sender that doesn't
/// zero them still means the text it encoded.
const BASE64: GeneralPurpose = GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::Indifferent)
        .with_decode_allow_trailing_bits(true),
);

/// What a body means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parsed {
    /// `?`: feature detection, answered with [`super::QUERY_REPLY`].
    Query,
    Report(Report),
}

/// The body of an `OSC 7501 ; …` sequence that ended with a terminator of `terminator_len` bytes
/// (1 for BEL, 2 for `ESC \`): the limit on the whole sequence counts both. `None` for anything
/// the terminal ignores.
pub fn parse(body: &[u8], terminator_len: usize) -> Option<Parsed> {
    if PREFIX_LEN + body.len() + terminator_len > SEQUENCE_MAX {
        return None;
    }
    if body.trim_ascii() == b"?" {
        return Some(Parsed::Query);
    }
    report(body).map(Parsed::Report)
}

/// The value of the last well-formed pair of each key the protocol knows.
#[derive(Default)]
struct Pairs<'a> {
    state: Option<&'a [u8]>,
    id: Option<&'a [u8]>,
    kind: Option<&'a [u8]>,
    progress: Option<&'a [u8]>,
    app: Option<&'a [u8]>,
    title: Option<&'a [u8]>,
    msg: Option<&'a [u8]>,
}

fn report(body: &[u8]) -> Option<Report> {
    let mut pairs = Pairs::default();
    for pair in body.split(|&byte| byte == b':') {
        // No `=`: there is no key to speak of, so the pair is skipped.
        let Some(eq) = pair.iter().position(|&byte| byte == b'=') else { continue };
        let (key, value) = (pair[..eq].trim_ascii(), pair[eq + 1..].trim_ascii());
        if key.len() > KEY_MAX {
            return None;
        }
        if key.is_empty() || !key.iter().all(u8::is_ascii_lowercase) {
            continue;
        }
        // A byte outside the value set makes a pair malformed, and a malformed pair is skipped,
        // except an id: the report must not fall back to the root record, which it could then
        // overwrite (the id check below fails on such a value).
        if !value.iter().all(|&byte| value_byte(byte)) && key != b"id" {
            continue;
        }
        match key {
            b"state" => pairs.state = Some(value),
            b"id" => pairs.id = Some(value),
            b"kind" => pairs.kind = Some(value),
            b"progress" => pairs.progress = Some(value),
            b"app" => pairs.app = Some(value),
            b"title" | b"msg" => {
                // The encoded size is checked before anything is decoded, for every pair.
                let max = if key == b"msg" { MSG_ENCODED_MAX } else { TITLE_ENCODED_MAX };
                if value.len() > max {
                    return None;
                }
                if key == b"msg" {
                    pairs.msg = Some(value);
                } else {
                    pairs.title = Some(value);
                }
            }
            _ => {}
        }
    }

    let state = pairs.state?;
    let id = match pairs.id {
        None => None,
        Some(id) => Some(id_of(id)?),
    };
    let title = text(pairs.title, TITLE_DECODED_MAX)?;
    let msg = text(pairs.msg, MSG_DECODED_MAX)?;
    let state = match state {
        b"idle" => State::Idle,
        b"working" => State::Working,
        b"done" => State::Done,
        b"blocked" => State::Blocked,
        b"error" => State::Error,
        b"clear" => return Some(Report::Clear { id }),
        // A state a later revision adds must not turn into idle on this terminal.
        _ => return None,
    };
    let kind = pairs.kind.and_then(kind_of).filter(|_| state == State::Blocked);
    let progress =
        pairs.progress.and_then(progress_of).filter(|_| matches!(state, State::Working | State::Blocked));
    let app = pairs.app.and_then(app_of);
    Some(Report::Set { id, record: Record { state, kind, progress, app, title, msg } })
}

fn value_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b',' | b'+' | b'/' | b'=' | b'-')
}

/// The characters of an id's segments and of `app`.
fn name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'+' | b'-')
}

/// `segment ("/" segment)*`, within the limits; `None` is a report to ignore.
fn id_of(id: &[u8]) -> Option<String> {
    if id.len() > ID_MAX {
        return None;
    }
    for (depth, segment) in id.split(|&byte| byte == b'/').enumerate() {
        if depth >= ID_DEPTH_MAX || !(1..=SEGMENT_MAX).contains(&segment.len()) {
            return None;
        }
        if !segment.iter().all(|&byte| name_byte(byte)) {
            return None;
        }
    }
    String::from_utf8(id.to_vec()).ok()
}

fn kind_of(value: &[u8]) -> Option<Kind> {
    match value {
        b"permission" => Some(Kind::Permission),
        b"question" => Some(Kind::Question),
        b"auth" => Some(Kind::Auth),
        _ => None,
    }
}

/// An integer from 0 to 100; anything else is no progress.
fn progress_of(value: &[u8]) -> Option<u8> {
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let number =
        value.iter().fold(0u32, |n, digit| n.saturating_mul(10).saturating_add(u32::from(digit - b'0')));
    u8::try_from(number).ok().filter(|&n| n <= 100)
}

fn app_of(value: &[u8]) -> Option<String> {
    if !(1..=APP_MAX).contains(&value.len()) || !value.iter().all(|&byte| name_byte(byte)) {
        return None;
    }
    String::from_utf8(value.to_vec()).ok()
}

/// A free-text value: base64 of UTF-8 without control characters, no longer than `decoded_max`.
/// `None` is a report to discard; `Some(None)` a value that says nothing (absent or empty).
fn text(value: Option<&[u8]>, decoded_max: usize) -> Option<Option<String>> {
    let Some(value) = value else { return Some(None) };
    let bytes = BASE64.decode(value).ok()?;
    if bytes.len() > decoded_max {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    // `is_control` is exactly U+0000–001F and U+007F–009F.
    if text.chars().any(char::is_control) {
        return None;
    }
    Some((!text.is_empty()).then_some(text))
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    use super::super::{Kind, Record, Report, State};
    use super::{PREFIX_LEN, Parsed, SEQUENCE_MAX, parse};

    /// A body ended by `ESC \`, as most programs send it.
    fn body(body: &str) -> Option<Parsed> {
        parse(body.as_bytes(), 2)
    }

    fn report(text: &str) -> Option<Report> {
        match body(text)? {
            Parsed::Report(report) => Some(report),
            Parsed::Query => None,
        }
    }

    /// The record a report sets at the root.
    fn record(text: &str) -> Record {
        match report(text) {
            Some(Report::Set { id: None, record }) => record,
            other => panic!("{text:?} is not a root record: {other:?}"),
        }
    }

    fn b64(text: &str) -> String {
        STANDARD.encode(text)
    }

    #[test]
    fn a_report_is_pairs_with_a_required_state() {
        let working = record("state=working");
        assert_eq!(
            working,
            Record { state: State::Working, kind: None, progress: None, app: None, title: None, msg: None }
        );
        for (word, state) in [
            ("idle", State::Idle),
            ("working", State::Working),
            ("done", State::Done),
            ("blocked", State::Blocked),
            ("error", State::Error),
        ] {
            assert_eq!(record(&format!("state={word}")).state, state);
        }
        assert_eq!(report("state=clear"), Some(Report::Clear { id: None }));
        assert_eq!(report("state=clear:id=a/b"), Some(Report::Clear { id: Some("a/b".into()) }));
    }

    #[test]
    fn a_missing_or_unknown_state_ignores_the_whole_report() {
        for text in [
            "",
            "app=cargo",
            "state=",
            "state=paused",
            "state=Working",
            "state=working!",
            "State=working",
            "app=cargo:msg=aGk=",
        ] {
            assert_eq!(report(text), None, "{text:?}");
        }
        // Not a report at all: no body.
        assert_eq!(body(""), None);
    }

    #[test]
    fn whitespace_is_trimmed_and_malformed_pairs_are_skipped() {
        let spaced = record("  state = working : app = cargo ");
        assert_eq!((spaced.state, spaced.app.as_deref()), (State::Working, Some("cargo")));
        // No `=`, an empty key, a key outside [a-z]+, a byte outside the value set, spaces inside
        // a value: each pair is skipped and the rest of the report is still processed.
        let rest = "state=working:oops:=x:Bad=1:a1=2:progress=5 5:app=été:kind=auth:msg=aGk!:title=aGk=";
        let skipped = record(rest);
        assert_eq!(skipped.state, State::Working);
        assert_eq!((skipped.progress, skipped.app, skipped.msg), (None, None, None));
        assert_eq!(skipped.title.as_deref(), Some("hi"), "a later pair after the malformed ones counts");
        // Not even a lone `:` or `=` breaks it.
        assert_eq!(record("::state=idle::=:").state, State::Idle);
    }

    #[test]
    fn unknown_keys_are_ignored_and_the_last_repeat_wins() {
        let record = record("state=working:future=whatever:app=one:other=:app=two");
        assert_eq!(record.app.as_deref(), Some("two"));
        assert_eq!(self::record("state=working:state=idle").state, State::Idle);
        // A repeat that is malformed is skipped, so the earlier one stands.
        assert_eq!(self::record("state=working:state=idle!").state, State::Working);
        assert_eq!(self::record("state=working:app=one:app=bad name").app.as_deref(), Some("one"));
        // The last `state` decides, even to a state this terminal doesn't know.
        assert_eq!(report("state=working:state=paused"), None);
    }

    #[test]
    fn ids_follow_the_segment_grammar() {
        let id = |text: &str| match report(&format!("state=working:id={text}")) {
            Some(Report::Set { id, .. }) => id,
            other => panic!("{text:?}: {other:?}"),
        };
        assert_eq!(id("build"), Some("build".into()));
        assert_eq!(id("build/test"), Some("build/test".into()));
        assert_eq!(id("A_b.c+d-e/0"), Some("A_b.c+d-e/0".into()));
        // No id at all is the root; an id that doesn't fit is a report to ignore, never the root.
        assert_eq!(record("state=working").state, State::Working);
        for bad in ["", "/", "/a", "a/", "a//b", "a b", "a,b", "a=b", "a%20b", "é", "a/é", "a b/c", "a\u{7f}"]
        {
            assert_eq!(report(&format!("state=working:id={bad}")), None, "{bad:?}");
            assert_eq!(report(&format!("state=clear:id={bad}")), None, "clear {bad:?}");
        }
        // The first id is fine, but the last one decides.
        assert_eq!(report("state=working:id=a:id=b b"), None);
        assert_eq!(id("x"), Some("x".into()));
    }

    #[test]
    fn id_limits_sit_exactly_at_their_edges() {
        let ok = |id: &str| matches!(report(&format!("state=idle:id={id}")), Some(Report::Set { .. }));
        let segment = "s".repeat(32);
        assert!(ok(&segment));
        assert!(!ok(&"s".repeat(33)), "33 bytes in a segment");
        assert!(ok(&["a"; 8].join("/")), "8 levels");
        assert!(!ok(&["a"; 9].join("/")), "9 levels");
        // 128 bytes in all: 3 segments of 32 and their 2 slashes are 98; 4 of 32 are 131.
        let id_of_len = |len: usize| {
            let mut id = String::new();
            while id.len() < len {
                if !id.is_empty() {
                    id.push('/');
                }
                let room = (len - id.len()).min(32);
                id.push_str(&"x".repeat(room));
            }
            id
        };
        assert_eq!(id_of_len(128).len(), 128);
        assert!(ok(&id_of_len(128)), "128 bytes");
        assert!(!ok(&id_of_len(129)), "129 bytes");
    }

    #[test]
    fn kind_belongs_to_blocked_and_progress_to_working_and_blocked() {
        let blocked = record("state=blocked:kind=permission:progress=40");
        assert_eq!((blocked.kind, blocked.progress), (Some(Kind::Permission), Some(40)));
        assert_eq!(record("state=blocked:kind=question").kind, Some(Kind::Question));
        assert_eq!(record("state=blocked:kind=auth").kind, Some(Kind::Auth));
        // Ignored where it doesn't belong, absent where it isn't understood.
        for state in ["idle", "working", "done", "error"] {
            assert_eq!(record(&format!("state={state}:kind=auth")).kind, None, "{state}");
        }
        assert_eq!(record("state=blocked:kind=something").kind, None);
        assert_eq!(record("state=blocked:kind=").kind, None);
        for state in ["idle", "done", "error"] {
            assert_eq!(record(&format!("state={state}:progress=50")).progress, None, "{state}");
        }
        assert_eq!(record("state=working:progress=50").progress, Some(50));
    }

    #[test]
    fn progress_is_an_integer_from_0_to_100_or_absent() {
        let progress = |value: &str| record(&format!("state=working:progress={value}")).progress;
        assert_eq!(progress("0"), Some(0));
        assert_eq!(progress("100"), Some(100));
        assert_eq!(progress("007"), Some(7));
        for bad in ["101", "1000", "-1", "+5", "5.5", "5%", "abc", "", "99999999999999999999"] {
            assert_eq!(progress(bad), None, "{bad:?}");
        }
        // Absent means indeterminate, which a report with no progress key is.
        assert_eq!(record("state=working").progress, None);
    }

    #[test]
    fn app_is_a_short_name_or_absent() {
        let app = |value: &str| record(&format!("state=working:app={value}")).app;
        assert_eq!(app("claude-code"), Some("claude-code".into()));
        assert_eq!(app("a_b.c+d-e"), Some("a_b.c+d-e".into()));
        assert_eq!(app(&"a".repeat(32)), Some("a".repeat(32)));
        // Outside [A-Za-z0-9_.+-]{1,32} the key is dropped and the report still applies.
        assert_eq!(app(&"a".repeat(33)), None);
        for bad in ["", "a b", "a,b", "a/b", "a=b", "é"] {
            assert_eq!(app(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn free_text_is_base64_with_optional_padding() {
        let record = record(&format!("state=done:msg={}:title={}", b64("Upgraded 12 packages"), b64("brew")));
        assert_eq!(record.msg.as_deref(), Some("Upgraded 12 packages"));
        assert_eq!(record.title.as_deref(), Some("brew"));
        // Padding may be left out, or kept: "hi" is "aGk=", "h" is "aA==".
        assert_eq!(self::record("state=done:msg=aGk").msg.as_deref(), Some("hi"));
        assert_eq!(self::record("state=done:msg=aGk=").msg.as_deref(), Some("hi"));
        assert_eq!(self::record("state=done:msg=aA").msg.as_deref(), Some("h"));
        assert_eq!(self::record("state=done:msg=aA==").msg.as_deref(), Some("h"));
        // UTF-8 beyond ASCII is text like any other, and so is a no-break space.
        assert_eq!(
            self::record(&format!("state=done:msg={}", b64("Ünï ✓ a\u{a0}b"))).msg.as_deref(),
            Some("Ünï ✓ a\u{a0}b")
        );
        // Nothing to say is the same as no msg.
        assert_eq!(self::record("state=done:msg=").msg, None);
    }

    #[test]
    fn text_that_does_not_decode_discards_the_whole_report() {
        let invalid_utf8 = STANDARD.encode([0x66, 0xff, 0x6f]);
        for bad in ["a", "aGk==", "a=Gk", "-_-_", "a_b-", "aGkaa", invalid_utf8.as_str()] {
            assert_eq!(report(&format!("state=working:msg={bad}")), None, "msg {bad:?}");
            assert_eq!(report(&format!("state=working:title={bad}")), None, "title {bad:?}");
        }
        // Spaces and other bytes outside the value set make the pair malformed instead: skipped,
        // and the report applies without it.
        for malformed in ["aG k", "aGk!", "aGk\u{e9}"] {
            assert_eq!(record(&format!("state=working:msg={malformed}")).msg, None, "{malformed:?}");
        }
    }

    #[test]
    fn a_control_character_in_the_decoded_text_discards_the_whole_report() {
        for ch in
            ['\u{0}', '\u{1}', '\n', '\t', '\r', '\u{1b}', '\u{1f}', '\u{7f}', '\u{80}', '\u{85}', '\u{9f}']
        {
            let text = format!("ab{ch}cd");
            assert_eq!(report(&format!("state=working:msg={}", b64(&text))), None, "msg {ch:?}");
            assert_eq!(report(&format!("state=working:title={}", b64(&text))), None, "title {ch:?}");
            assert_eq!(report(&format!("state=clear:msg={}", b64(&text))), None, "clear {ch:?}");
        }
        // The characters on either side of the control ranges are text.
        for ch in ['\u{20}', '\u{7e}', '\u{a0}', '\u{2028}', '\u{202e}'] {
            let text = format!("ab{ch}cd");
            assert!(report(&format!("state=working:msg={}", b64(&text))).is_some(), "{ch:?}");
        }
    }

    #[test]
    fn text_sizes_sit_exactly_at_their_edges() {
        let msg = |len: usize| report(&format!("state=working:msg={}", b64(&"m".repeat(len))));
        let title = |len: usize| report(&format!("state=working:title={}", b64(&"t".repeat(len))));
        // msg: 2048 decoded bytes are 2732 encoded (with padding); one more is over 2048 decoded.
        assert!(msg(2048).is_some());
        assert_eq!(b64(&"m".repeat(2048)).len(), 2732);
        assert_eq!(msg(2049), None);
        // title: 192 decoded are 256 encoded; 193 are over both.
        assert!(title(192).is_some());
        assert_eq!(b64(&"t".repeat(192)).len(), 256);
        assert_eq!(title(193), None);
        // The encoded size is a limit of its own, checked before anything is decoded: one more
        // character than the longest legal encoding of either.
        let long = |key: &str, len: usize| report(&format!("state=working:{key}={}", "A".repeat(len)));
        assert_eq!(long("msg", 2733), None);
        assert_eq!(long("title", 257), None);
        // The limit holds for a pair a later one overrides.
        assert_eq!(report(&format!("state=working:msg={}:msg=aGk", "A".repeat(2733))), None);
    }

    #[test]
    fn keys_are_at_most_16_bytes() {
        assert!(report(&format!("state=working:{}=1", "k".repeat(16))).is_some());
        assert_eq!(report(&format!("state=working:{}=1", "k".repeat(17))), None);
        // Counted after the whitespace around it is removed.
        assert!(report(&format!("state=working: {} =1", "k".repeat(16))).is_some());
    }

    #[test]
    fn the_whole_sequence_is_at_most_4096_bytes() {
        // ESC ] 7501 ; <body> ST: 7 bytes before the body, 1 (BEL) or 2 (ESC \) after it.
        let padded = |len: usize| {
            let mut text = String::from("state=working:x=");
            text.push_str(&"y".repeat(len - text.len()));
            text
        };
        assert_eq!(PREFIX_LEN, "\u{1b}]7501;".len());
        for (terminator, longest) in [(2, SEQUENCE_MAX - 7 - 2), (1, SEQUENCE_MAX - 7 - 1)] {
            let fits = padded(longest);
            assert!(parse(fits.as_bytes(), terminator).is_some(), "{longest} bytes, terminator {terminator}");
            let over = padded(longest + 1);
            assert_eq!(
                parse(over.as_bytes(), terminator),
                None,
                "{} bytes, terminator {terminator}",
                longest + 1
            );
        }
        // The query is no exception.
        assert_eq!(parse(b"?", 2), Some(Parsed::Query));
        let spaced = format!("?{}", " ".repeat(SEQUENCE_MAX));
        assert_eq!(parse(spaced.as_bytes(), 2), None);
    }

    #[test]
    fn feature_detection_is_a_lone_question_mark() {
        assert_eq!(body("?"), Some(Parsed::Query));
        assert_eq!(body(" ? "), Some(Parsed::Query));
        for other in ["??", "?:states=idle", "x?", "?=", "state=?"] {
            assert!(!matches!(body(other), Some(Parsed::Query)), "{other:?}");
        }
    }

    #[test]
    fn bytes_that_are_not_text_do_not_get_in_the_way() {
        let mut raw = b"state=working:app=cargo:junk=".to_vec();
        raw.extend_from_slice(&[0xff, 0xfe, 0x80]);
        raw.extend_from_slice(b":title=aGk=");
        let Some(Parsed::Report(Report::Set { record, .. })) = parse(&raw, 2) else { panic!("not a report") };
        assert_eq!(
            (record.state, record.app.as_deref(), record.title.as_deref()),
            (State::Working, Some("cargo"), Some("hi"))
        );
    }
}
