//! What the OSC sequences alacritty ignores say: the shell's folder (OSC 7, ConEmu's OSC 9;9),
//! FinalTerm's semantic prompt marks (OSC 133), a program's notification (OSC 9, OSC 777, kitty's
//! OSC 99), its progress (OSC 9;4) and its status (OSC 7501, `program_status`). This is parsing
//! only, from the `;`-separated parameters `vte` hands over; `osc_tap` runs it on the PTY thread
//! and acts on the result.
//!
//! Everything here is untrusted program output: text is stripped of control and formatting
//! characters and cut to a length a banner can show, paths must decode to UTF-8, numbers are
//! bounded, and kitty's chunked notifications are kept to a few small pending ones.

use std::path::PathBuf;

use base64::Engine;

use crate::terminal::program_status::{self, Parsed};

/// One sequence the terminal acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Osc {
    /// OSC 7 `file://host/path`, or OSC 9;9 (ConEmu, no host): where the shell says it is. `host`
    /// is as written; whether it is this machine is decided where the hostname is known.
    Cwd {
        host: String,
        path: PathBuf,
    },
    /// OSC 133, and the number of the prompt it belongs to (the tap counts them; `parse` leaves
    /// 0, which is "before any prompt").
    Mark(Mark, u64),
    Notify(Notification),
    /// OSC 9;4: `None` removes the progress.
    Progress(Option<Progress>),
    /// OSC 8 with an empty URI: the program ended its hyperlink (the tap re-arms a prompt mark).
    HyperlinkEnd,
    /// OSC 7501: a program reports its status.
    ProgramStatus(program_status::Report),
    /// OSC 7501 `?`: a program asks whether the terminal speaks the protocol.
    ProgramStatusQuery,
    /// RIS (`ESC c`): a full reset. The tap sees it where alacritty resets the screen. (A soft
    /// reset, DECSTR, is not one.)
    FullReset,
}

/// FinalTerm's semantic prompt marks (OSC 133), as shells with integration send them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// `A`: a prompt starts here.
    PromptStart(PromptKind),
    /// `B`: the prompt ended; what follows is the command line being typed.
    CommandStart,
    /// `C`: the command runs; what follows is its output.
    OutputStart,
    /// `D[;code]`: the command finished with this exit code (`None` when none ran, as after an
    /// empty line, or when the shell gave none).
    Finished(Option<i32>),
}

/// Which prompt an `A` mark starts (`k=`): the one a command is typed at, or one of the extra
/// prompts around it, which belong to the same command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptKind {
    Initial,
    /// A later line of a multi-line command (`k=s`).
    Secondary,
    /// A right-aligned prompt (`k=r`).
    Right,
    /// A continuation line (`k=c`).
    Continuation,
}

/// A program's own notification (OSC 9, 777, 99): cleaned text, at least one line of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    /// Empty when the program gave only a body (OSC 9).
    pub title: String,
    pub body: String,
}

/// A program's progress (OSC 9;4, ConEmu / Windows Terminal / Ghostty).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pub state: ProgressState,
    /// 0–100; 0 when the state has no percentage (indeterminate).
    pub percent: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressState {
    Normal,
    Error,
    Indeterminate,
    Paused,
}

/// Most a notification's title and body may be, in characters (a banner shows about that much).
pub const TITLE_MAX_CHARS: usize = 100;
pub const BODY_MAX_CHARS: usize = 400;
/// Most one kitty notification may accumulate per line while chunks (`d=0`) come in.
const KITTY_CHUNK_MAX_BYTES: usize = 4096;
/// How many kitty notifications may wait for their last chunk at once.
const KITTY_PENDING_MAX: usize = 4;

/// Parse one OSC's parameters (`params[0]` is its number). `None` for anything the terminal
/// doesn't act on, including what alacritty handles itself. Kitty's chunked notifications
/// accumulate in `kitty` and come out with their last chunk. `bell`: the sequence ended with BEL
/// rather than `ESC \` (OSC 7501 limits the sequence's length, terminator included).
pub(super) fn parse(params: &[&[u8]], bell: bool, kitty: &mut KittyChunks) -> Option<Osc> {
    match params {
        [b"7", rest @ ..] => cwd(&join(rest)),
        [b"8", _, uri @ ..] if join(uri).is_empty() => Some(Osc::HyperlinkEnd),
        [b"9", b"4", state, rest @ ..] => progress(state, rest.first().copied()).map(Osc::Progress),
        [b"9", b"9", path @ ..] => {
            let path = join(path);
            let path = path.strip_prefix('"').and_then(|p| p.strip_suffix('"')).unwrap_or(&path);
            local_path(path).map(|path| Osc::Cwd { host: String::new(), path })
        }
        // ConEmu's other numbered commands (sleep, message box, tab title, prompt marks, …).
        [b"9", command, ..] if conemu_command(command) => None,
        [b"9", text @ ..] if !text.is_empty() => notification("", &join(text)).map(Osc::Notify),
        [b"99", metadata, payload @ ..] => kitty.chunk(metadata, &join(payload)).map(Osc::Notify),
        [b"133", mark, rest @ ..] => self::mark(mark, rest).map(|mark| Osc::Mark(mark, 0)),
        [b"777", b"notify", title, body @ ..] => notification(&text(title), &join(body)).map(Osc::Notify),
        [b"7501", body @ ..] if !body.is_empty() => {
            // Bytes, not text: the protocol's characters are ASCII and anything else must be seen
            // as the bytes it is (and counted as many).
            match program_status::parse(&join_bytes(body), if bell { 1 } else { 2 })? {
                Parsed::Query => Some(Osc::ProgramStatusQuery),
                Parsed::Report(report) => Some(Osc::ProgramStatus(report)),
            }
        }
        _ => None,
    }
}

/// [`join`] without making text of it.
fn join_bytes(params: &[&[u8]]) -> Vec<u8> {
    params.join(&b';')
}

/// The parameters after a split at `;`, joined again: the payload may contain `;` itself.
fn join(params: &[&[u8]]) -> String {
    let mut out = String::new();
    for (i, param) in params.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        out.push_str(&text(param));
    }
    out
}

fn text(param: &[u8]) -> String {
    String::from_utf8_lossy(param).into_owned()
}

/// ConEmu uses `OSC 9 ; n ; …` with a small `n` for its own commands; iTerm2's notification is
/// `OSC 9 ; text`. A text that is a one- or two-digit number is taken for ConEmu's.
fn conemu_command(param: &[u8]) -> bool {
    (1..=2).contains(&param.len()) && param.iter().all(u8::is_ascii_digit)
}

/// OSC 7's URL: `file://host/path` (percent-encoded), `file:///path`, or a bare absolute path.
fn cwd(url: &str) -> Option<Osc> {
    let (host, path) = match url.strip_prefix("file://") {
        Some(rest) => {
            let at = rest.find('/')?;
            (&rest[..at], &rest[at..])
        }
        None if url.starts_with('/') => ("", url),
        None => return None,
    };
    let host = percent_decode(host.as_bytes())?;
    let path = percent_decode(path.as_bytes())?;
    local_path(&path).map(|path| Osc::Cwd { host, path })
}

/// An absolute path without NUL or control characters, as a `PathBuf`; the tap checks it exists.
fn local_path(path: &str) -> Option<PathBuf> {
    (path.starts_with('/') && !path.chars().any(char::is_control)).then(|| PathBuf::from(path))
}

/// `%XX` decoded; `None` when the result isn't UTF-8 (a path that isn't text isn't a folder we
/// can show).
pub(crate) fn percent_decode(bytes: &[u8]) -> Option<String> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 3;
            }
            b'%' => return None,
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

fn mark(mark: &[u8], rest: &[&[u8]]) -> Option<Mark> {
    let value = |key: &[u8]| rest.iter().find_map(|param| param.strip_prefix(key));
    Some(match mark {
        b"A" => Mark::PromptStart(match value(b"k=") {
            Some(b"s") => PromptKind::Secondary,
            Some(b"r") => PromptKind::Right,
            Some(b"c") => PromptKind::Continuation,
            _ => PromptKind::Initial,
        }),
        b"B" => Mark::CommandStart,
        b"C" => Mark::OutputStart,
        b"D" => {
            let code =
                rest.first().and_then(|code| std::str::from_utf8(code).ok()).and_then(|c| c.parse().ok());
            Mark::Finished(code)
        }
        _ => return None,
    })
}

/// `OSC 9 ; 4 ; state ; percent`: 0 removes it, 1 normal, 2 error, 3 indeterminate, 4 paused.
fn progress(state: &[u8], percent: Option<&[u8]>) -> Option<Option<Progress>> {
    let state = match state {
        b"0" => return Some(None),
        b"1" => ProgressState::Normal,
        b"2" => ProgressState::Error,
        b"3" => ProgressState::Indeterminate,
        b"4" => ProgressState::Paused,
        _ => return None,
    };
    let percent = percent
        .and_then(|p| std::str::from_utf8(p).ok())
        .and_then(|p| p.trim().parse::<u32>().ok())
        .map_or(0, |p| p.min(100) as u8);
    Some(Some(Progress { state, percent }))
}

/// A notification from a program's title and body, cleaned; `None` when nothing is left to show.
fn notification(title: &str, body: &str) -> Option<Notification> {
    let title = clean(title, TITLE_MAX_CHARS);
    let body = clean(body, BODY_MAX_CHARS);
    (!title.is_empty() || !body.is_empty()).then_some(Notification { title, body })
}

/// Program text as a banner may show it: on one line, whitespace runs folded, without control
/// characters or the invisible formatting ones (bidi overrides, zero-width joiners, soft hyphens)
/// that could disguise what it says, and cut to `max_chars` with an ellipsis.
pub fn clean(raw: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for c in raw.chars() {
        if c.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if c.is_control() || is_invisible(c) {
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(c);
    }
    if let Some((at, _)) = out.char_indices().nth(max_chars) {
        out.truncate(at);
        out.push('…');
    }
    out
}

/// Unicode formatting characters that show nothing themselves but change how what follows reads.
fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
    )
}

/// Kitty's notifications (OSC 99) arrive as `metadata ; payload`, the metadata `key=value` pairs
/// joined by `:`. A notification may come in several chunks (`d=0` until the last) and in two
/// parts (`p=title`, `p=body`), tied together by `i=<id>`; `e=1` says the payload is base64.
#[derive(Debug, Default)]
pub(super) struct KittyChunks {
    pending: Vec<KittyPending>,
}

#[derive(Debug)]
struct KittyPending {
    id: String,
    title: String,
    body: String,
}

impl KittyChunks {
    /// One OSC 99; the notification once its last chunk is in.
    fn chunk(&mut self, metadata: &[u8], payload: &str) -> Option<Notification> {
        let metadata = text(metadata);
        let value = |key: &str| {
            metadata.split(':').find_map(|pair| pair.strip_prefix(key).and_then(|v| v.strip_prefix('=')))
        };
        let id = value("i").unwrap_or("0");
        let done = value("d") != Some("0");
        let payload = match value("e") {
            Some("1") => base64::engine::general_purpose::STANDARD
                .decode(payload)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default(),
            _ => payload.to_string(),
        };
        let at = match self.pending.iter().position(|p| p.id == id) {
            Some(at) => at,
            None if self.pending.len() < KITTY_PENDING_MAX => {
                self.pending.push(KittyPending {
                    id: id.to_string(),
                    title: String::new(),
                    body: String::new(),
                });
                self.pending.len() - 1
            }
            // Too many half-sent notifications: the oldest one makes room.
            None => {
                self.pending.remove(0);
                self.pending.push(KittyPending {
                    id: id.to_string(),
                    title: String::new(),
                    body: String::new(),
                });
                self.pending.len() - 1
            }
        };
        let pending = &mut self.pending[at];
        match value("p") {
            None | Some("title") => append_capped(&mut pending.title, &payload),
            Some("body") => append_capped(&mut pending.body, &payload),
            // Queries, icons, buttons, closing: not supported; a query gets no answer, which
            // kitty's protocol reads as "not supported".
            Some(_) => {
                if done {
                    self.pending.remove(at);
                }
                return None;
            }
        }
        if !done {
            return None;
        }
        let KittyPending { title, body, .. } = self.pending.remove(at);
        notification(&title, &body)
    }
}

fn append_capped(target: &mut String, more: &str) {
    let room = KITTY_CHUNK_MAX_BYTES.saturating_sub(target.len());
    let mut take = more.len().min(room);
    while !more.is_char_boundary(take) {
        take -= 1;
    }
    target.push_str(&more[..take]);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        BODY_MAX_CHARS, KittyChunks, Mark, Notification, Osc, Progress, ProgressState, PromptKind,
        TITLE_MAX_CHARS, clean, parse,
    };
    use crate::terminal::program_status::{Record, Report, State};

    fn osc(params: &[&str]) -> Option<Osc> {
        let params: Vec<&[u8]> = params.iter().map(|p| p.as_bytes()).collect();
        parse(&params, false, &mut KittyChunks::default())
    }

    fn notify(title: &str, body: &str) -> Option<Osc> {
        Some(Osc::Notify(Notification { title: title.into(), body: body.into() }))
    }

    #[test]
    fn osc_7_gives_the_host_and_the_decoded_path() {
        assert_eq!(
            osc(&["7", "file://studio.local/Users/me/My%20Code"]),
            Some(Osc::Cwd { host: "studio.local".into(), path: PathBuf::from("/Users/me/My Code") })
        );
        assert_eq!(
            osc(&["7", "file:///tmp"]),
            Some(Osc::Cwd { host: String::new(), path: PathBuf::from("/tmp") })
        );
        // A bare path, as some shells send it.
        assert_eq!(osc(&["7", "/srv"]), Some(Osc::Cwd { host: String::new(), path: PathBuf::from("/srv") }));
        // The path may contain `;`, which vte split off.
        assert_eq!(
            osc(&["7", "file://h/a", "b"]),
            Some(Osc::Cwd { host: "h".into(), path: PathBuf::from("/a;b") })
        );
        for bad in [
            "file://host",
            "relative/path",
            "file://h/%zz",
            "file://h/%ff%fe",
            "/a\u{1b}[31mb",
            "",
            "file://",
        ] {
            assert_eq!(osc(&["7", bad]), None, "{bad:?}");
        }
    }

    #[test]
    fn conemus_cwd_is_a_local_path_with_optional_quotes() {
        let local = |p: &str| Some(Osc::Cwd { host: String::new(), path: PathBuf::from(p) });
        assert_eq!(osc(&["9", "9", "/opt/x"]), local("/opt/x"));
        assert_eq!(osc(&["9", "9", "\"/opt/y\""]), local("/opt/y"));
        assert_eq!(osc(&["9", "9", "C:\\x"]), None);
    }

    #[test]
    fn the_prompt_marks_and_the_exit_code() {
        let mark = |mark| Some(Osc::Mark(mark, 0));
        assert_eq!(osc(&["133", "A"]), mark(Mark::PromptStart(PromptKind::Initial)));
        assert_eq!(osc(&["133", "A", "k=i"]), mark(Mark::PromptStart(PromptKind::Initial)));
        assert_eq!(osc(&["133", "A", "aid=7", "k=s"]), mark(Mark::PromptStart(PromptKind::Secondary)));
        assert_eq!(osc(&["133", "A", "k=r"]), mark(Mark::PromptStart(PromptKind::Right)));
        assert_eq!(osc(&["133", "A", "k=c"]), mark(Mark::PromptStart(PromptKind::Continuation)));
        assert_eq!(osc(&["133", "B"]), mark(Mark::CommandStart));
        assert_eq!(osc(&["133", "C"]), mark(Mark::OutputStart));
        assert_eq!(osc(&["133", "D"]), mark(Mark::Finished(None)));
        assert_eq!(osc(&["133", "D", "0"]), mark(Mark::Finished(Some(0))));
        assert_eq!(osc(&["133", "D", "130", "aid=1"]), mark(Mark::Finished(Some(130))));
        assert_eq!(osc(&["133", "D", "lots"]), mark(Mark::Finished(None)));
        assert_eq!(osc(&["133", "D", "99999999999"]), mark(Mark::Finished(None)));
        assert_eq!(osc(&["133", "P", "k=i"]), None, "kitty's extensions are left alone");
        assert_eq!(osc(&["133"]), None);
    }

    #[test]
    fn progress_states_and_removal() {
        let progress = |state, percent| Some(Osc::Progress(Some(Progress { state, percent })));
        assert_eq!(osc(&["9", "4", "1", "42"]), progress(ProgressState::Normal, 42));
        assert_eq!(osc(&["9", "4", "2", "250"]), progress(ProgressState::Error, 100));
        assert_eq!(osc(&["9", "4", "3"]), progress(ProgressState::Indeterminate, 0));
        assert_eq!(osc(&["9", "4", "4", "x"]), progress(ProgressState::Paused, 0));
        assert_eq!(osc(&["9", "4", "0"]), Some(Osc::Progress(None)));
        assert_eq!(osc(&["9", "4", "7", "1"]), None);
    }

    #[test]
    fn iterm2_and_rxvt_notifications() {
        assert_eq!(osc(&["9", "Build finished"]), notify("", "Build finished"));
        assert_eq!(osc(&["9", "a", "b"]), notify("", "a;b"), "the text may contain `;`");
        assert_eq!(osc(&["9", "12"]), None, "ConEmu's prompt mark, not a notification");
        assert_eq!(osc(&["9", "2", "debug box"]), None);
        assert_eq!(osc(&["9", ""]), None);
        assert_eq!(osc(&["9"]), None);
        assert_eq!(osc(&["777", "notify", "Deploy", "done in 3m"]), notify("Deploy", "done in 3m"));
        assert_eq!(osc(&["777", "notify", "Deploy"]), notify("Deploy", ""));
        assert_eq!(osc(&["777", "notify", "", "  "]), None);
        assert_eq!(osc(&["777", "other", "x"]), None);
    }

    #[test]
    fn kitty_notifications_simple_chunked_and_two_part() {
        let mut kitty = KittyChunks::default();
        let send = |kitty: &mut KittyChunks, params: &[&str]| {
            let params: Vec<&[u8]> = params.iter().map(|p| p.as_bytes()).collect();
            parse(&params, false, kitty)
        };
        assert_eq!(send(&mut kitty, &["99", "", "hello"]), notify("hello", ""));
        // Title in two chunks, then the body ends it.
        assert_eq!(send(&mut kitty, &["99", "i=1:d=0", "Hel"]), None);
        assert_eq!(send(&mut kitty, &["99", "i=1:d=0", "lo"]), None);
        assert_eq!(send(&mut kitty, &["99", "i=1:p=body", "World"]), notify("Hello", "World"));
        // Another notification's chunks in between don't mix.
        assert_eq!(send(&mut kitty, &["99", "i=a:d=0", "A"]), None);
        assert_eq!(send(&mut kitty, &["99", "i=b:d=0", "B"]), None);
        assert_eq!(send(&mut kitty, &["99", "i=b", "!"]), notify("B!", ""));
        assert_eq!(send(&mut kitty, &["99", "i=a", ""]), notify("A", ""));
        // base64 payloads, and a payload with `;`.
        assert_eq!(send(&mut kitty, &["99", "e=1", "aGk="]), notify("hi", ""));
        assert_eq!(send(&mut kitty, &["99", "", "a", "b"]), notify("a;b", ""));
        // A query or a close is not a notification and leaves nothing pending.
        assert_eq!(send(&mut kitty, &["99", "i=q:p=?", ""]), None);
        assert_eq!(send(&mut kitty, &["99", "i=q", "after"]), notify("after", ""));
        assert!(kitty.pending.is_empty());
    }

    #[test]
    fn kitty_chunks_are_bounded() {
        let mut kitty = KittyChunks::default();
        let big = "x".repeat(3000);
        for _ in 0..10 {
            let params: Vec<&[u8]> = vec![b"99", b"i=1:d=0", big.as_bytes()];
            assert_eq!(parse(&params, false, &mut kitty), None);
        }
        assert!(kitty.pending[0].title.len() <= 4096);
        // More pending ids than allowed: the oldest makes room.
        for id in ["i=2:d=0", "i=3:d=0", "i=4:d=0", "i=5:d=0"] {
            let params: Vec<&[u8]> = vec![b"99", id.as_bytes(), b"t"];
            parse(&params, false, &mut kitty);
        }
        assert_eq!(kitty.pending.len(), 4);
        assert_eq!(kitty.pending[0].id, "2");
        let params: Vec<&[u8]> = vec![b"99", b"i=1", b"gone"];
        assert_eq!(parse(&params, false, &mut kitty), notify("gone", ""), "id 1 starts over");
    }

    #[test]
    fn program_text_is_cleaned_and_cut() {
        assert_eq!(clean("  Build\n\tdone \x07 now\u{85} ", 100), "Build done now");
        // vte ends a string at ESC, so what follows one never gets here as part of it; a stray
        // control character is simply dropped.
        assert_eq!(clean("a\x1bb", 100), "ab");
        assert_eq!(clean("a\u{202E}b\u{200B}c\u{00AD}d", 100), "abcd", "bidi and zero-width marks go");
        assert_eq!(clean("Ünïcödé ✓", 100), "Ünïcödé ✓");
        let long = "x".repeat(150);
        let cut = clean(&long, TITLE_MAX_CHARS);
        assert_eq!(cut.chars().count(), TITLE_MAX_CHARS + 1);
        assert!(cut.ends_with('…'));
        let huge = "y ".repeat(1000);
        assert_eq!(clean(&huge, BODY_MAX_CHARS).chars().count(), BODY_MAX_CHARS + 1);
        assert_eq!(clean("\u{1b}\u{7f}\u{9b}", 10), "");
    }

    #[test]
    fn program_status_reports_and_queries() {
        let record = Record {
            state: State::Working,
            kind: None,
            progress: None,
            app: Some("brew".into()),
            title: None,
            msg: Some("hi".into()),
        };
        assert_eq!(
            osc(&["7501", "state=working:app=brew:msg=aGk="]),
            Some(Osc::ProgramStatus(Report::Set { id: None, record }))
        );
        assert_eq!(
            osc(&["7501", "state=clear:id=a/b"]),
            Some(Osc::ProgramStatus(Report::Clear { id: Some("a/b".into()) }))
        );
        assert_eq!(osc(&["7501", "?"]), Some(Osc::ProgramStatusQuery));
        // Nothing to report, or nothing the protocol knows.
        for none in [&["7501"][..], &["7501", ""], &["7501", "app=brew"], &["7501", "state=nope"]] {
            assert_eq!(osc(none), None, "{none:?}");
        }
        // vte splits at `;`, which the protocol has no use for: joined again it is part of a pair's
        // value, and a byte outside the value set makes the pair malformed.
        assert_eq!(osc(&["7501", "state=working", "app=brew"]), None);
        assert_eq!(osc(&["7501", "app=brew", "state=idle"]), None);
    }

    #[test]
    fn program_status_counts_its_terminator_in_the_sequence_limit() {
        let body = format!("state=idle:x={}", "y".repeat(4096 - 7 - 2 - "state=idle:x=".len()));
        let params: Vec<&[u8]> = vec![b"7501", body.as_bytes()];
        let report = |bell| parse(&params, bell, &mut KittyChunks::default());
        assert!(report(false).is_some(), "ESC \\ takes 2 bytes: 4096 in all");
        let longer = format!("{body}y");
        let params: Vec<&[u8]> = vec![b"7501", longer.as_bytes()];
        assert!(parse(&params, true, &mut KittyChunks::default()).is_some(), "BEL takes 1: 4096 in all");
        assert!(parse(&params, false, &mut KittyChunks::default()).is_none(), "ESC \\: 4097");
    }
}
