//! What the source lints share: where the sources are and the string literals in them. Each test
//! file includes it with `mod support;` and uses part of it.
#![allow(dead_code, reason = "each test file uses part of this")]

use std::path::{Path, PathBuf};

pub(crate) fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

pub(crate) fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read source dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// A file of tests only, which `#[cfg(test)] mod x;` pulls in: one named so, or one in a folder of
/// tests (`flow_tests/details.rs`).
pub(crate) fn is_test_file(relative: &str) -> bool {
    let mut parts = relative.rsplit('/');
    let name = parts.next().unwrap_or(relative);
    name == "tests.rs"
        || name.ends_with("_tests.rs")
        || parts.any(|folder| folder == "tests" || folder.ends_with("_tests"))
}

/// The part of a file that is not test code: up to a `#[cfg(test)]`-style attribute (one that
/// names `test`) in front of an inline module, which closes the file.
pub(crate) fn non_test_part(text: &str) -> &str {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(") && trimmed.contains("test") {
            // The attribute is followed (perhaps after more attributes) by the item it gates.
            let rest = &text[offset + line.len()..];
            let item = rest.lines().map(str::trim_start).find(|l| !l.starts_with("#["));
            if item.is_some_and(|l| (l.starts_with("mod ") || l.starts_with("pub mod ")) && l.ends_with('{'))
            {
                return &text[..offset];
            }
        }
        offset += line.len();
    }
    text
}

pub(crate) fn all_sources() -> Vec<PathBuf> {
    let mut files = Vec::new();
    sources(&src(), &mut files);
    files.sort();
    files
}

/// A string literal in a Rust source, with the line it starts on.
pub(crate) struct Literal {
    pub(crate) line: usize,
    pub(crate) text: String,
    /// It is an argument of a `tracing` macro (a log line, not something on screen).
    pub(crate) logged: bool,
    /// The code of the statement before the literal, from its start: what the literal is an argument of.
    pub(crate) before: String,
}

/// The string literals of a Rust source (plain and raw), skipping comments and char literals.
pub(crate) fn string_literals(source: &str) -> Vec<Literal> {
    const LOG_MACROS: [&str; 6] = ["tracing::", "trace!(", "debug!(", "info!(", "warn!(", "error!("];
    let chars: Vec<char> = source.chars().collect();
    let (mut found, mut i, mut line, mut statement) = (Vec::new(), 0, 1, 0);
    let at = |i: usize, s: &str| s.chars().enumerate().all(|(k, c)| chars.get(i + k) == Some(&c));
    while i < chars.len() {
        let c = chars[i];
        let ident_before = i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
        if at(i, "//") {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if at(i, "/*") {
            while i < chars.len() && !at(i, "*/") {
                line += usize::from(chars[i] == '\n');
                i += 1;
            }
            i += 2;
            continue;
        }
        let raw_hashes = (c == 'r' && !ident_before)
            .then(|| chars[i + 1..].iter().take_while(|&&h| h == '#').count())
            .filter(|&n| chars.get(i + 1 + n) == Some(&'"'));
        if c == '"' || raw_hashes.is_some() {
            let start_line = line;
            let before: String = chars[statement..i].iter().collect();
            let logged = LOG_MACROS.iter().any(|m| before.contains(m));
            let mut text = String::new();
            match raw_hashes {
                Some(n) => {
                    let close: String = std::iter::once('"').chain(std::iter::repeat_n('#', n)).collect();
                    i += n + 2;
                    while i < chars.len() && !at(i, &close) {
                        line += usize::from(chars[i] == '\n');
                        text.push(chars[i]);
                        i += 1;
                    }
                    i += close.chars().count();
                }
                None => {
                    i += 1;
                    while i < chars.len() && chars[i] != '"' {
                        if chars[i] == '\\' {
                            text.push(chars[i]);
                            i += 1;
                        }
                        line += usize::from(chars.get(i) == Some(&'\n'));
                        text.extend(chars.get(i));
                        i += 1;
                    }
                    i += 1;
                }
            }
            found.push(Literal { line: start_line, text, logged, before });
            continue;
        }
        if c == '\'' {
            // A char literal ('x', '\n', '"'); anything else is a lifetime.
            if chars.get(i + 1) == Some(&'\\') {
                i += 2;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
            } else if chars.get(i + 2) == Some(&'\'') {
                i += 2;
            }
            i += 1;
            continue;
        }
        if matches!(c, ';' | '{' | '}') {
            statement = i + 1;
        }
        line += usize::from(c == '\n');
        i += 1;
    }
    found
}
