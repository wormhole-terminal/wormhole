//! Workspace trust, the rules: nothing executable from a project folder runs until the user approved the
//! exact contents of everything that can make the app run or type something.
//!
//! Approval is bound to a fingerprint of those inputs, so a `git pull` that changes `actions.json`
//! (or a prompt file an action submits) turns the project from `Trusted` into `Changed` and the
//! user has to re-approve. The fingerprint and the content that runs come from one
//! [`ProjectSnapshot`](super::ProjectSnapshot), so they can't belong to different versions of the files.
//!
//! This file is the rules and what they are made of: the states, the fingerprint's inputs and how they
//! are hashed, and the lines of an approval dialog. They know nothing of a snapshot or of the store:
//! the snapshot (`snapshot`) is built from them and says what its content would ask, and the store
//! (`trust_store`) keeps what was approved and compares it to a snapshot.
//!
//! `config.json` is fingerprinted by what it says, not by its bytes, and without the two keys that
//! only paint (`project_color` and its lock): see [`config_input`]. Choosing a colour in the app
//! therefore doesn't send a trusted project back for approval. Approvals given before that, under
//! the bytes' fingerprint (v2), still hold for exactly the content they covered, and are rewritten
//! in the new form (v3) the next time the project is loaded.

use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;

use serde_json::Value;

use super::ProjectError;
use super::color::PAINT_KEYS;
// The rule itself is shared with Probes (`storage::fingerprint`); what is hashed, and under which tag, stays here.
use crate::storage::fingerprint::Input;

/// Bumping this invalidates every stored approval (they show as `Changed` once), e.g. when the
/// set of inputs changes, unless the old one is still understood (see [`LEGACY_VERSION`]). v2 added
/// the prompt files that actions paste; v3 fingerprints `config.json` by its parsed content
/// without the keys that only paint. The value keeps the codename the app had until 0.2: it goes
/// into every stored approval's hash, so renaming it would ask to trust every project again.
pub(super) const FINGERPRINT_VERSION: &str = "andromeda-trust-v3";
/// The fingerprint approvals were stored under until `config.json` was taken by what it says:
/// the same inputs, `config.json` as its raw bytes. A stored approval equal to it is for
/// exactly the bytes now on disk, so [`TrustStore::status_of`](super::TrustStore::status_of) still accepts it.
pub(super) const LEGACY_VERSION: &str = "andromeda-trust-v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustStatus {
    Untrusted,
    Trusted,
    /// Was trusted, but the files changed since: must be re-approved before use.
    Changed,
}

/// The input for a file the loader read (or failed to), by its text.
pub(super) fn input_of(read: &Result<Option<String>, ProjectError>) -> Input {
    match read {
        Ok(None) => Input::Absent,
        Ok(Some(text)) => Input::of_bytes(text.as_bytes()),
        Err(_) => Input::Unreadable,
    }
}

/// The input for `config.json`: what the file says, as one canonical text, without the top-level
/// keys that only paint ([`PAINT_KEYS`]); `parsed` is the file's text read as JSON by the loader.
/// The same parse that the project's settings, environment and shell come from is hashed, so
/// the fingerprint covers exactly what the loader gets out of the file:
///
/// - Anything that changes a value, a key, an array's order or the nesting changes the text, so
///   it needs approving again (`env`, `shell`, a setting, a key a later version will read).
/// - Comments, white space and the order of an object's keys don't reach the loader, can't run
///   anything, and so don't count. (The text is canonical: object keys are sorted, no spaces.)
/// - Only the top level's two paint keys are left out; the same names deeper down (inside `env`)
///   are data and count.
///
/// A file that doesn't parse has no canonical form and is taken as its raw bytes, as before. It
/// can't be approved while it fails to load, and any edit to it changes the result.
pub(super) fn config_input(
    read: &Result<Option<String>, ProjectError>,
    parsed: Option<&Result<Value, ProjectError>>,
) -> Input {
    let Some(Ok(value)) = parsed else { return input_of(read) };
    let mut canonical = String::new();
    match value {
        Value::Object(map) => {
            write_object(map.iter().filter(|(key, _)| !PAINT_KEYS.contains(&key.as_str())), &mut canonical);
        }
        other => write_value(other, &mut canonical),
    }
    Input::of_bytes(canonical.as_bytes())
}

/// `value` as JSON text with every object's keys in sorted order (they are unique after a parse)
/// and no white space: strings, numbers and literals as `serde_json` writes them.
fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => write_object(map.iter(), out),
        Value::Array(items) => {
            out.push('[');
            for (ix, item) in items.iter().enumerate() {
                if ix > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        scalar => out.push_str(&scalar.to_string()),
    }
}

fn write_object<'a>(members: impl Iterator<Item = (&'a String, &'a Value)>, out: &mut String) {
    let mut members: Vec<_> = members.collect();
    members.sort_by(|a, b| a.0.cmp(b.0));
    out.push('{');
    for (ix, (key, value)) in members.into_iter().enumerate() {
        if ix > 0 {
            out.push(',');
        }
        out.push_str(&Value::String(key.clone()).to_string());
        out.push(':');
        write_value(value, out);
    }
    out.push('}');
}

/// One line of the approval dialog: what runs it (the shell setting, an env var, an action with
/// its key, a layout terminal), what it runs or types, and where or how (the target, a folder),
/// kept apart so the dialog can set what runs in its own colour. Written out, it reads
/// "action test (key ⌃⌘T): run `cargo test` (current)".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustLine {
    pub what: String,
    pub runs: String,
    pub note: String,
}

impl TrustLine {
    pub(super) fn new(what: impl Into<String>, runs: impl Into<String>, note: impl Into<String>) -> Self {
        Self { what: what.into(), runs: runs.into(), note: note.into() }
    }

    /// The line with every [`hidden`] character written out.
    pub(super) fn revealed(&self) -> Self {
        Self::new(reveal(&self.what), reveal(&self.runs), reveal(&self.note))
    }
}

impl fmt::Display for TrustLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.what)?;
        if !self.runs.is_empty() {
            write!(f, " `{}`", self.runs)?;
        }
        if !self.note.is_empty() {
            write!(f, " {}", self.note)?;
        }
        Ok(())
    }
}

/// What the approval dialog shows: everything that would be enabled by trusting the folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustRequest {
    pub root: PathBuf,
    /// Fingerprint of the contents described here. Pass the request to
    /// [`TrustStore::trust_request`](super::TrustStore::trust_request) so a file changed while the dialog was open isn't approved.
    pub fingerprint: String,
    /// Present trust-relevant files (prompt files that actions paste included).
    pub files: Vec<PathBuf>,
    /// One line per shell, env var, action and layout command, with characters that could hide
    /// part of a line made visible.
    pub commands: Vec<TrustLine>,
}

/// Characters that would hide part of a command in the approval dialog while still reaching the
/// shell or the agent: line breaks and other controls (terminal escapes), bidi overrides that
/// reorder what is shown ("Trojan Source"), zero-width and other invisible characters, and
/// Unicode tag characters, which render as nothing but which language models read as text
/// ("ASCII smuggling").
pub(super) fn hidden(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2069}')
        || matches!(c, '\u{061c}' | '\u{2028}' | '\u{2029}' | '\u{feff}' | '\u{00ad}' | '\u{180e}')
        || matches!(c, '\u{115f}' | '\u{1160}' | '\u{3164}' | '\u{ffa0}')
        || matches!(c, '\u{fe00}'..='\u{fe0f}' | '\u{e0000}'..='\u{e007f}' | '\u{e0100}'..='\u{e01ef}')
}

/// `line` with every [`hidden`] character written out (`\n`, `\t`, `\u{202e}`).
fn reveal(line: &str) -> Cow<'_, str> {
    if !line.chars().any(hidden) {
        return Cow::Borrowed(line);
    }
    let mut out = String::with_capacity(line.len() + 8);
    for c in line.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if hidden(c) => out.push_str(&format!("\\u{{{:x}}}", u32::from(c))),
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}
