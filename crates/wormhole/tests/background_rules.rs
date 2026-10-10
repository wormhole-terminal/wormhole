//! Source lint that keeps background mode working. A test instance started with
//! `WORMHOLE_BACKGROUND=1` stays behind the other apps' windows only because every activation
//! and every new window goes through `src/background.rs` (`activate`, `activate_window`,
//! `open_window`). Code that calls GPUI's own `activate(true)`, `activate_window()` or
//! `open_window` pops the instance up over whoever is at the Mac, and nothing else notices: the
//! instance still works. So this reads the sources as text (like `design_rules.rs`, which is for
//! the design language and so not the place for it) and fails on such a call.
//!
//! Allowed: `background.rs` itself; test code (files named `*_tests.rs` or `tests.rs`, and a
//! `#[cfg(test)]` module closing a file), whose windows belong to GPUI's test platform; and
//! devctl's `front`, which brings a window forward on purpose (see [`ALLOWED`]).

use std::path::{Path, PathBuf};

/// The calls that bypass `background.rs`, as they appear in code. (`open_window(` is checked on
/// its own below, as `background::open_window(` is the right call.)
const ACTIVATIONS: [&str; 3] = [".activate(true)", ".activate(false)", ".activate_window()"];

/// Direct uses that are meant, by file (relative to `src/`), pattern and how many there are. A
/// further one in the same file fails, and so does a file that stops using them (the entry is then
/// stale).
const ALLOWED: [(&str, &str, usize); 2] = [
    // devctl's `front`: "bring this window to the front with the keys, background mode or not".
    ("devctl/sessions.rs", ".activate(true)", 1),
    ("devctl/sessions.rs", ".activate_window()", 1),
];

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read source dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// A file of tests only, which `#[cfg(test)] mod x;` pulls in.
fn is_test_file(relative: &str) -> bool {
    let name = relative.rsplit('/').next().unwrap_or(relative);
    name == "tests.rs" || name.ends_with("_tests.rs")
}

/// The part of a file that is not test code: up to a `#[cfg(test)]`-style attribute (one that
/// names `test`) in front of an inline module, which closes the file.
fn non_test_part(text: &str) -> &str {
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

/// What `line` (comments cut) does that background mode can't have: the pattern, and for
/// `open_window(` anything but the wrapper.
fn bypasses(line: &str) -> Vec<&'static str> {
    let code = line.split("//").next().unwrap_or("");
    let mut found: Vec<&'static str> = ACTIVATIONS.iter().copied().filter(|p| code.contains(p)).collect();
    let direct_open = code.match_indices("open_window(").any(|(at, _)| {
        let before = &code[..at];
        !before.ends_with("background::") && !before.trim_end().ends_with("fn")
    });
    if direct_open {
        found.push("open_window(");
    }
    found
}

/// Every bypass in `text` of the file at `relative` (under `src/`): line and pattern.
fn violations(relative: &str, text: &str) -> Vec<(usize, &'static str)> {
    if relative == "background.rs" || is_test_file(relative) {
        return Vec::new();
    }
    non_test_part(text)
        .lines()
        .enumerate()
        .flat_map(|(n, line)| bypasses(line).into_iter().map(move |pattern| (n + 1, pattern)))
        .collect()
}

#[test]
fn activation_and_new_windows_go_through_background_mode() {
    let root = src();
    let mut files = Vec::new();
    sources(&root, &mut files);
    files.sort();
    let mut found = Vec::new();
    let mut allowed_seen = [0usize; ALLOWED.len()];
    for file in files {
        let relative = file.strip_prefix(&root).expect("under src").to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(&file).expect("read source");
        for (line, pattern) in violations(&relative, &text) {
            let allowed = ALLOWED.iter().position(|(f, p, _)| *f == relative && *p == pattern);
            match allowed {
                Some(ix) => {
                    allowed_seen[ix] += 1;
                    if allowed_seen[ix] > ALLOWED[ix].2 {
                        found.push(format!("{relative}:{line}: {pattern} (one more than allowed)"));
                    }
                }
                None => found.push(format!("{relative}:{line}: {pattern}")),
            }
        }
    }
    for (ix, (file, pattern, count)) in ALLOWED.iter().enumerate() {
        if allowed_seen[ix] < *count {
            found.push(format!(
                "{file}: expected {count} of {pattern}, found {} (update ALLOWED)",
                allowed_seen[ix]
            ));
        }
    }
    assert!(
        found.is_empty(),
        "background mode keeps a test instance behind other apps only if every activation and window \
         goes through crate::background (activate, activate_window, open_window):\n{}",
        found.join("\n")
    );
}

#[test]
fn the_lint_sees_what_it_is_for() {
    let bad = "fn f(cx: &mut App, window: &Window) {\n    cx.activate(true);\n    window.activate_window();\n    \
               let _ = gpui_kit::open_window(o, cx, build);\n    let _ = open_window(o, cx, build);\n    \
               let _ = cx.open_window(o, build);\n}\n";
    let lines: Vec<usize> = violations("workspace/x.rs", bad).into_iter().map(|(line, _)| line).collect();
    assert_eq!(lines, [2, 3, 4, 5, 6]);

    let fine = "fn f(cx: &mut App, window: &Window) {\n    crate::background::activate(cx);\n    \
                background::activate_window(window, cx);\n    \
                let _ = crate::background::open_window(o, cx, build);\n    \
                // cx.activate(true) in a comment\n    self.activate(1, window, cx);\n}\n";
    assert!(violations("workspace/x.rs", fine).is_empty());
}

#[test]
fn background_rs_and_test_code_are_left_alone() {
    let call = "fn f(cx: &mut App) {\n    cx.activate(true);\n}\n";
    assert!(violations("background.rs", call).is_empty(), "the wrapper itself");
    assert!(violations("workspace/view_tests.rs", call).is_empty(), "a tests file");
    assert!(violations("select/tests.rs", call).is_empty());

    let inline = format!(
        "{call}\n#[cfg(all(test, feature = \"devctl\"))]\nmod tests {{\n    fn t(cx: &mut App) {{\n        \
                          cx.activate(true);\n    }}\n}}\n"
    );
    assert!(
        violations("style/x.rs", &inline).iter().all(|(line, _)| *line == 2),
        "only the code before the module"
    );
    assert_eq!(violations("style/x.rs", &inline).len(), 1);

    // A gated item that is not a module doesn't end the code.
    let gated = "#[cfg(test)]\nfn helper() {}\nfn f(cx: &mut App) {\n    cx.activate(true);\n}\n";
    assert_eq!(violations("style/x.rs", gated), [(4, ".activate(true)")]);
}
