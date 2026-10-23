//! Source lint for the interface's words: in the folders listed in [`MIGRATED`], no sentence or label
//! is written in Rust. Every word a person reads is a message of `wormhole-text`'s catalogs
//! (`crates/wormhole-text/locales/`), so it can be translated, checked and shown in the
//! pseudo-language. `MIGRATED` *is* the progress of that move: a folder or file joins it when it is done,
//! and nothing leaves it.
//!
//! What counts as text: a string literal with a run of two letters or more that has a space in it
//! or starts with a capital (`{…}` placeholders aside). So `"Cancel"` and `"Open {}"` fail, and the
//! ids and keys of code (`"setting-{}"`, `"hotkey"`) do not. Left alone: log lines (`tracing` and
//! `trace!`/`debug!`/`info!`/`warn!`/`error!`), `assert*`, `expect`, `panic!`, `unreachable!`, `env!` and
//! `include_str!`, `#[…]` attributes, test code (files named `tests.rs` or `*_tests.rs`, a `#[cfg(test)]`
//! module closing a file), devctl, and a line that ends in a comment saying why it is not text:
//! `// text: not shown (a key context)`, or `// text: sample (what the preview's terminal prints)` for
//! the made-up content of a preview, which is not translated.
//!
//! Also failing in these folders: the old ways of building a plural (`count(n, "terminal", "terminals")`
//! of `sidebar::text`, `if n == 1 { "" } else { "s" }`): a plural is a Fluent message with a select.
//!
//! Blind spots, said plainly: text from user data is not a literal; a literal made by `concat!` is not
//! scanned; one word in lower case (`"off"`) is not told from an id, and the pseudo-language
//! (`devctl language en-XA`: a label without accents or brackets is still hard-coded) catches what
//! this misses. Text built from pieces is caught only by its literal parts.

mod support;

use support::{Literal, is_test_file, non_test_part, sources, src, string_literals};

/// Folders and files under `src/` whose words are all in the catalogs.
const MIGRATED: &[&str] = &[];

/// A marker that says a literal on its line is not interface text.
const MARKER: &str = "// text:";

/// `{…}` placeholders cut out, what a reader would see of a format string.
fn without_placeholders(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for c in text.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            c if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// Whether a literal reads as text to a person: two letters in a row, and a space or a capital first.
fn reads_as_text(literal: &str) -> bool {
    let text = without_placeholders(literal);
    let mut run = 0;
    let has_word = text.chars().any(|c| {
        run = if c.is_alphabetic() { run + 1 } else { 0 };
        run >= 2
    });
    let first_letter = text.chars().find(|c| c.is_alphabetic());
    has_word && (text.contains(char::is_whitespace) || first_letter.is_some_and(char::is_uppercase))
}

/// Whether the literal is an argument of something that is not shown: an assertion, a panic message,
/// `expect`, a compile-time include, an attribute's reason or a log line.
fn is_not_shown(literal: &Literal) -> bool {
    let before = literal.before.trim_end();
    const CALLS: [&str; 7] =
        ["expect(", "panic!(", "unreachable!(", "todo!(", "include_str!(", "env!(", "option_env!("];
    let attribute =
        before.rsplit('\n').next().is_some_and(|line| line.contains("#[")) || before.ends_with("reason =");
    literal.logged
        || attribute
        || CALLS.iter().any(|call| before.ends_with(call))
        || ["assert!(", "assert_eq!(", "assert_ne!(", "debug_assert!("].iter().any(|a| before.contains(a))
}

/// Whether the line `n` (1-based) of `code` carries the marker.
fn marked(code: &str, n: usize) -> bool {
    code.lines().nth(n - 1).is_some_and(|line| line.contains(MARKER))
}

/// The ways to make a plural in code, which a Fluent select replaces.
fn old_plural(line: &str) -> Option<&'static str> {
    let code = line.split("//").next().unwrap_or("");
    if code.contains("== 1 {") || code.contains("!= 1 {") {
        return Some("a plural by `if n == 1`");
    }
    if code.contains("else { \"s\" }") || code.contains("else { \"es\" }") {
        return Some("a plural by a suffix");
    }
    let calls_count = code.match_indices("count(").any(|(at, _)| {
        let before = code[..at].chars().next_back();
        before.is_none_or(|c| !(c == '.' || c == '_' || c.is_alphanumeric()))
            && !code[at..].starts_with("count()")
    });
    calls_count.then_some("`sidebar::text::count`")
}

/// What is written in Rust in the file at `relative` (under `src/`): line and what it is.
fn violations(relative: &str, text: &str) -> Vec<(usize, String)> {
    let devctl = relative.split('/').any(|part| part == "devctl" || part == "devctl.rs");
    if is_test_file(relative) || devctl {
        return Vec::new();
    }
    let code = non_test_part(text);
    let mut found: Vec<(usize, String)> = string_literals(code)
        .into_iter()
        .filter(|literal| {
            reads_as_text(&literal.text) && !is_not_shown(literal) && !marked(code, literal.line)
        })
        .map(|literal| (literal.line, format!("{:?}", literal.text)))
        .collect();
    for (n, line) in code.lines().enumerate() {
        if let Some(what) = old_plural(line) {
            found.push((n + 1, what.to_string()));
        }
    }
    found.sort();
    found
}

/// The files under `src/` that `entry` (a folder or a file) names.
fn files_of(entry: &str) -> Vec<(String, std::path::PathBuf)> {
    let root = src();
    let path = root.join(entry);
    let mut files = Vec::new();
    if path.is_dir() {
        sources(&path, &mut files);
    } else if path.is_file() {
        files.push(path);
    }
    files.sort();
    files
        .into_iter()
        .map(|file| {
            let relative = file.strip_prefix(&root).expect("under src").to_string_lossy().replace('\\', "/");
            (relative, file)
        })
        .collect()
}

#[test]
fn the_migrated_folders_have_their_words_in_the_catalogs() {
    let mut found = Vec::new();
    for entry in MIGRATED {
        let files = files_of(entry);
        assert!(!files.is_empty(), "MIGRATED names {entry}, which is not under src/");
        for (relative, file) in files {
            let text = std::fs::read_to_string(&file).expect("read source");
            for (line, what) in violations(&relative, &text) {
                found.push(format!("{relative}:{line}: {what}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "words written in Rust: make each a message in crates/wormhole-text/locales/en/ (or, if it is \
         not shown, say why in a `// text: not shown (why)` comment):\n{}",
        found.join("\n")
    );
}

#[test]
fn text_is_told_from_ids_and_placeholders() {
    for text in [
        "Cancel",
        "Open {}",
        "{} spaces",
        "Show all {total}",
        "cargo test",
        "Open in a new terminal",
        "“{}” is not NAME=value",
    ] {
        assert!(reads_as_text(text), "{text:?}");
    }
    for text in [
        "setting-{}",
        "hotkey",
        "{}",
        "{n}%",
        "w:{}x{}",
        "",
        "a",
        "profile-delete-cancel",
        "--flag",
        "{:.2}",
        "ok",
    ] {
        assert!(!reads_as_text(text), "{text:?}");
    }
}

#[test]
fn the_lint_finds_words_in_code_and_leaves_the_rest() {
    let code = "fn f() {\n    label(\"Cancel\");\n    tracing::warn!(\"Couldn't read {}\", x);\n    \
                let v = y.expect(\"the row exists\");\n    assert!(ok, \"Not ok\");\n    \
                id(\"profile-delete\");\n    ctx.key_context(\"SettingsWindow\"); // text: not shown (a key context)\n    \
                #[allow(dead_code, reason = \"Only a test uses it\")]\n    let n = items.len();\n}\n";
    let found = violations("settings_window/x.rs", code);
    assert_eq!(found, [(2, "\"Cancel\"".to_string())]);
}

#[test]
fn tests_and_devctl_are_left_alone() {
    let code = "fn f() {\n    label(\"Cancel\");\n}\n";
    assert!(violations("settings_window/devctl.rs", code).is_empty());
    assert!(violations("settings_window/tests.rs", code).is_empty());
    assert!(violations("settings_window/x_tests.rs", code).is_empty());
    let inline = format!("{code}\n#[cfg(test)]\nmod tests {{\n    fn t() {{ label(\"Other\"); }}\n}}\n");
    assert_eq!(violations("settings_window/x.rs", &inline).len(), 1, "only the code before the module");
}

#[test]
fn the_old_plural_idioms_are_found() {
    let code = "fn f(n: usize) {\n    let a = count(n, \"terminal\", \"terminals\");\n    \
                let b = format!(\"{n} font{}\", if n == 1 { \"\" } else { \"s\" });\n    \
                let c = items.iter().count();\n    let d = row_count(n);\n}\n";
    let found = violations("settings_window/x.rs", code);
    let lines: Vec<usize> = found.iter().map(|(line, _)| *line).collect();
    assert!(lines.contains(&2) && lines.contains(&3), "{found:?}");
    assert!(!lines.contains(&4) && !lines.contains(&5), "{found:?}");
}
