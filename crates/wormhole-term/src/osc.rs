//! What the engine hands the host raw, as core understands it: the shell's folder (OSC 7,
//! ConEmu's OSC 9;9), FinalTerm's semantic prompt marks (OSC 133), a program's notification
//! (OSC 9, OSC 777, kitty's OSC 99), its progress (OSC 9;4) and its status (OSC 7501,
//! `program_status`). The engine reads the sequences themselves (`engine::host` turns its events
//! into these); this is the policy on top.
//!
//! Everything here is untrusted program output: text is stripped of control and formatting
//! characters and cut to a length a banner can show, paths must decode to UTF-8, and kitty's
//! chunked notifications are kept to a few small pending ones.

use std::path::PathBuf;

use base64::Engine;

use crate::program_status;

/// One sequence the terminal acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Osc {
    /// OSC 7 `file://host/path`, or OSC 9;9 (ConEmu, no host): where the shell says it is. `host`
    /// is as written; whether it is this machine is decided where the hostname is known.
    Cwd {
        host: String,
        path: PathBuf,
    },
    /// OSC 133, and the number of the prompt it belongs to (the host counts them, 0 being "before
    /// any prompt").
    Mark(Mark, u64),
    Notify(Notification),
    /// OSC 9;4: `None` removes the progress.
    Progress(Option<Progress>),
    /// OSC 7501: a program reports its status.
    ProgramStatus(program_status::Report),
    /// OSC 7501 `?`: a program asks whether the terminal speaks the protocol.
    ProgramStatusQuery,
    /// RIS (`ESC c`): a full reset. (A soft reset, DECSTR, is not one.)
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

/// OSC 7's URL: `file://host/path` (percent-encoded), `file:///path`, or a bare absolute path.
pub(crate) fn cwd(url: &str) -> Option<Osc> {
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

/// An absolute path without NUL or control characters, as a `PathBuf`; whoever acts on it checks it exists.
fn local_path(path: &str) -> Option<PathBuf> {
    (path.starts_with('/') && !path.chars().any(char::is_control)).then(|| PathBuf::from(path))
}

/// `%XX` decoded; `None` when the result isn't UTF-8 (a path that isn't text isn't a folder we
/// can show) or a `%` is not followed by two hexadecimal digits (`from_str_radix` alone would
/// take `%+F` for 15).
pub fn percent_decode(bytes: &[u8]) -> Option<String> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let digits = &bytes[i + 1..i + 3];
                if !digits.iter().all(u8::is_ascii_hexdigit) {
                    return None;
                }
                let hex = std::str::from_utf8(digits).ok()?;
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

/// A notification from a program's title and body, cleaned; `None` when nothing is left to show.
pub(crate) fn notification(title: &str, body: &str) -> Option<Notification> {
    let title = clean(title, TITLE_MAX_CHARS);
    let body = clean(body, BODY_MAX_CHARS);
    (!title.is_empty() || !body.is_empty()).then_some(Notification { title, body })
}

/// Program text as a banner may show it: on one line, whitespace runs folded, without control
/// characters or the invisible formatting ones (bidi overrides, zero-width joiners, soft hyphens,
/// fillers, tag characters, variation selectors) that could disguise what it says or carry data
/// the reader cannot see, and cut to `max_chars` with an ellipsis.
pub fn clean(raw: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    // Whether the character last kept can take a variation selector (see `takes_selector`).
    let mut selectable = false;
    for c in raw.chars() {
        if c.is_whitespace() {
            pending_space = !out.is_empty();
            selectable = false;
            continue;
        }
        if !keeps(c, &mut selectable) {
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

/// Whether `clean` keeps the non-blank character `c`; `selectable` is whether the one kept before
/// it can take a presentation selector, and is updated.
fn keeps(c: char, selectable: &mut bool) -> bool {
    if c.is_control() || is_invisible(c) {
        return false;
    }
    if is_presentation_selector(c) {
        // Emoji and text presentation (✔︎ ✔️, 1️⃣) are shown, so one is kept right after a
        // character that can have it; any other is a hidden bit (or byte, in a run).
        if !*selectable {
            return false;
        }
        *selectable = false;
    } else {
        *selectable = takes_selector(c);
    }
    true
}

/// Whether `raw` has a character that does not show as itself: one `clean` would drop (control and
/// invisible formatting characters, a presentation selector with nothing to act on), or a blank
/// that is not the space, tab or newline it looks like (no-break and other Unicode spaces, which a
/// shell does not read as a space). For text that is about to be typed as it was shown, where
/// cleaning it would be a different command.
pub(crate) fn hides_text(raw: &str) -> bool {
    let mut selectable = false;
    raw.chars().any(|c| {
        if c.is_whitespace() {
            selectable = false;
            return !matches!(c, ' ' | '\t' | '\n');
        }
        !keeps(c, &mut selectable)
    })
}

/// Unicode characters that show nothing themselves: formatting ones that change how what follows
/// reads (bidi controls, joiners), blanks that take up room and look like nothing (the Hangul and
/// Mongolian fillers, the combining grapheme joiner), tags (a whole alphabet that renders as
/// nothing, used to hide text in an emoji) and the variation selectors, except the two that
/// choose between text and emoji presentation, which `clean` keeps where they have something to
/// act on (`is_presentation_selector`).
fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{034F}'
            | '\u{061C}'
            | '\u{115F}'
            | '\u{1160}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{3164}'
            | '\u{FE00}'..='\u{FE0D}'
            | '\u{FEFF}'
            | '\u{FFA0}'
            | '\u{E0000}'..='\u{E007F}'
            | '\u{E0100}'..='\u{E01EF}'
    )
}

/// VS15 and VS16: text and emoji presentation.
fn is_presentation_selector(c: char) -> bool {
    matches!(c, '\u{FE0E}' | '\u{FE0F}')
}

/// Whether a presentation selector after `c` means anything: a symbol or emoji, or a digit, `#` or
/// `*` (keycaps). Letters and ASCII punctuation never take one, so one there carries nothing a
/// reader sees.
fn takes_selector(c: char) -> bool {
    (!c.is_alphabetic() && !c.is_ascii_punctuation()) || matches!(c, '#' | '*')
}

/// Kitty's notifications (OSC 99) arrive as `metadata ; payload`, the metadata `key=value` pairs
/// joined by `:`. A notification may come in several chunks (`d=0` until the last) and in two
/// parts (`p=title`, `p=body`), tied together by `i=<id>`; `e=1` says the payload is base64.
#[derive(Debug, Default)]
pub(crate) struct KittyChunks {
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
    pub(crate) fn chunk(&mut self, metadata: &[u8], payload: &str) -> Option<Notification> {
        let metadata = String::from_utf8_lossy(metadata);
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

    use super::{BODY_MAX_CHARS, KittyChunks, Notification, Osc, TITLE_MAX_CHARS, clean, cwd};

    fn notify(title: &str, body: &str) -> Option<Notification> {
        Some(Notification { title: title.into(), body: body.into() })
    }

    /// One OSC 99 as the engine hands it over: the metadata and the payload, split at the first `;`.
    fn kitty(chunks: &mut KittyChunks, params: &str) -> Option<Notification> {
        let (metadata, payload) = params.split_once(';').unwrap_or((params, ""));
        chunks.chunk(metadata.as_bytes(), payload)
    }

    #[test]
    fn osc_7_gives_the_host_and_the_decoded_path() {
        assert_eq!(
            cwd("file://studio.local/Users/me/My%20Code"),
            Some(Osc::Cwd { host: "studio.local".into(), path: PathBuf::from("/Users/me/My Code") })
        );
        assert_eq!(cwd("file:///tmp"), Some(Osc::Cwd { host: String::new(), path: PathBuf::from("/tmp") }));
        // A bare path, as some shells send it, and ConEmu's OSC 9;9 (the engine hands it over alike).
        assert_eq!(cwd("/srv"), Some(Osc::Cwd { host: String::new(), path: PathBuf::from("/srv") }));
        // The path may contain `;`.
        assert_eq!(cwd("file://h/a;b"), Some(Osc::Cwd { host: "h".into(), path: PathBuf::from("/a;b") }));
        for bad in [
            "file://host",
            "relative/path",
            "file://h/%zz",
            "file://h/%ff%fe",
            "/a\u{1b}[31mb",
            "C:\\x",
            "",
            "file://",
        ] {
            assert_eq!(cwd(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn kitty_notifications_simple_chunked_and_two_part() {
        let mut chunks = KittyChunks::default();
        assert_eq!(kitty(&mut chunks, ";hello"), notify("hello", ""));
        // Title in two chunks, then the body ends it.
        assert_eq!(kitty(&mut chunks, "i=1:d=0;Hel"), None);
        assert_eq!(kitty(&mut chunks, "i=1:d=0;lo"), None);
        assert_eq!(kitty(&mut chunks, "i=1:p=body;World"), notify("Hello", "World"));
        // Another notification's chunks in between don't mix.
        assert_eq!(kitty(&mut chunks, "i=a:d=0;A"), None);
        assert_eq!(kitty(&mut chunks, "i=b:d=0;B"), None);
        assert_eq!(kitty(&mut chunks, "i=b;!"), notify("B!", ""));
        assert_eq!(kitty(&mut chunks, "i=a;"), notify("A", ""));
        // base64 payloads, and a payload with `;`.
        assert_eq!(kitty(&mut chunks, "e=1;aGk="), notify("hi", ""));
        assert_eq!(kitty(&mut chunks, ";a;b"), notify("a;b", ""));
        // A query or a close is not a notification and leaves nothing pending.
        assert_eq!(kitty(&mut chunks, "i=q:p=?;"), None);
        assert_eq!(kitty(&mut chunks, "i=q;after"), notify("after", ""));
        assert!(chunks.pending.is_empty());
    }

    #[test]
    fn kitty_chunks_are_bounded() {
        let mut chunks = KittyChunks::default();
        let big = "x".repeat(3000);
        for _ in 0..10 {
            assert_eq!(chunks.chunk(b"i=1:d=0", &big), None);
        }
        assert!(chunks.pending[0].title.len() <= 4096);
        // More pending ids than allowed: the oldest makes room.
        for id in ["i=2:d=0", "i=3:d=0", "i=4:d=0", "i=5:d=0"] {
            chunks.chunk(id.as_bytes(), "t");
        }
        assert_eq!(chunks.pending.len(), 4);
        assert_eq!(chunks.pending[0].id, "2");
        assert_eq!(chunks.chunk(b"i=1", "gone"), notify("gone", ""), "id 1 starts over");
    }

    #[test]
    fn program_text_is_cleaned_and_cut() {
        assert_eq!(clean("  Build\n\tdone \x07 now\u{85} ", 100), "Build done now");
        // A string ends at ESC, so what follows one never gets here as part of it; a stray
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
    fn invisible_characters_are_dropped_wherever_they_would_hide_something() {
        // Fillers and joiners that are blank without being space, an Arabic letter mark, and the
        // Mongolian vowel separator.
        for hidden in ['\u{061C}', '\u{180E}', '\u{115F}', '\u{1160}', '\u{3164}', '\u{FFA0}', '\u{034F}'] {
            assert_eq!(clean(&format!("a{hidden}b"), 10), "ab", "U+{:04X}", u32::from(hidden));
            assert_eq!(clean(&hidden.to_string(), 10), "", "U+{:04X} alone", u32::from(hidden));
        }
        // Tag characters spell text that no font draws: a whole message after an emoji.
        let tagged: String = "\u{1F600}"
            .chars()
            .chain("hide me".chars().map(|c| char::from_u32(0xE0000 + c as u32).unwrap_or(' ')))
            .collect();
        assert_eq!(clean(&tagged, 100), "\u{1F600}");
        assert_eq!(clean("a\u{E0001}\u{E007F}b", 100), "ab");
        // Variation selectors 1 to 14 and the supplement are never kept.
        assert_eq!(clean("a\u{FE00}b\u{FE0D}c\u{E0100}d\u{E01EF}e", 100), "abcde");
        assert_eq!(clean("\u{FE00}\u{FE0F}", 100), "");
    }

    #[test]
    fn text_hides_something_exactly_where_cleaning_it_would_change_it() {
        use super::hides_text;
        for shown in [
            "ls -l",
            "echo 'Ünïcödé ✓'",
            "a\tb",
            "one\ntwo",
            "done \u{2714}\u{FE0F}",
            "",
            "1\u{FE0F}\u{20E3}",
        ] {
            assert!(!hides_text(shown), "{shown:?}");
        }
        for hidden in [
            "a\u{200B}b",
            "a\u{202E}b",
            "a\u{E0041}b",
            "a\u{FE00}b",
            "a\u{FE0F}b",
            "a\x1bb",
            "a\rb",
            "a\u{3164}b",
        ] {
            assert!(hides_text(hidden), "{hidden:?}");
            assert_ne!(clean(hidden, 100), hidden, "cleaning it changes it: {hidden:?}");
        }
        // A blank that is not a space: it looks like one and a shell does not read it as one.
        assert!(hides_text("rm\u{00A0}-rf"));
        assert!(hides_text("a\u{2003}b"));
    }

    #[test]
    fn the_emoji_selector_stays_on_an_emoji_and_goes_everywhere_else() {
        // VS16 after a symbol, VS15 for the text form, a keycap: shown, so kept (once).
        assert_eq!(clean("done \u{2714}\u{FE0F}", 100), "done \u{2714}\u{FE0F}");
        assert_eq!(clean("\u{2714}\u{FE0E}", 100), "\u{2714}\u{FE0E}");
        assert_eq!(clean("1\u{FE0F}\u{20E3}", 100), "1\u{FE0F}\u{20E3}");
        // A run of them is a hidden message (one bit each); only the first has anything to act on.
        assert_eq!(clean("\u{2714}\u{FE0F}\u{FE0E}\u{FE0F}\u{FE0F}", 100), "\u{2714}\u{FE0F}");
        // After a letter, a space or at the start there is nothing to choose.
        assert_eq!(clean("a\u{FE0F}b\u{FE0E}", 100), "ab");
        assert_eq!(clean("a \u{FE0F}b", 100), "a b");
        assert_eq!(clean("\u{FE0F}x", 100), "x");
        assert_eq!(clean("!\u{FE0F}", 100), "!");
    }

    #[test]
    fn percent_decoding_wants_two_hexadecimal_digits() {
        use super::percent_decode;
        assert_eq!(percent_decode(b"a%20b").as_deref(), Some("a b"));
        assert_eq!(percent_decode(b"%41%4a%4A").as_deref(), Some("AJJ"));
        // `from_str_radix` takes a sign: `%+F` was 15, and `%-1` an error only by accident.
        for bad in [&b"%+F"[..], b"%+f", b"%-1", b"%F+", b"% 1", b"%\xe2\x82", b"%1", b"%", b"%zz"] {
            assert_eq!(percent_decode(bad), None, "{:?}", String::from_utf8_lossy(bad));
        }
        assert_eq!(cwd("file://h/a%+Fb"), None);
    }
}
