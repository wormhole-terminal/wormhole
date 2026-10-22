//! Golden snapshots of every built-in theme's resolved design tokens: each UI token,
//! syntax style and metric, one per line, and what the Increase Contrast variant changes. A
//! rule change anywhere in `tokens/` shows up here as a reviewable diff of exactly the
//! values it moved, theme by theme.
//!
//! After an intended change, regenerate the files and review the diff:
//! `UPDATE_GOLDEN=1 cargo test -p wormhole-core --test golden_tokens`.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

use wormhole_core::theme::tokens::{METRIC_TOKENS, ThemeTokens, Translucency, builtins};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/tokens")
}

/// `Catppuccin Latte` → `catppuccin-latte`.
fn file_name(theme: &str) -> String {
    let slug: String =
        theme.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    format!("{slug}.txt")
}

/// The window looks whose translucent colours are pinned too: the default
/// opacity with and without the blur.
const LOOKS: [(&str, Translucency); 2] = [
    ("80% blur", Translucency { opacity: 0.8, blur: true }),
    ("80% no blur", Translucency { opacity: 0.8, blur: false }),
];

/// UI tokens and syntax styles sorted by name, so a new token lands in one place and
/// moves nothing else; then what a translucent window draws in place of a few of them, and
/// the tokens and metrics the Increase Contrast variant changes.
fn render(t: &ThemeTokens) -> String {
    let mut out = format!("# {} ({:?})\n", t.name, t.appearance);
    for (name, color) in t.ui_tokens() {
        let _ = writeln!(out, "{name} {}", color.to_hex());
    }
    for (key, style) in &t.syntax {
        let flags = match (style.bold, style.italic) {
            (true, true) => " bold italic",
            (true, false) => " bold",
            (false, true) => " italic",
            (false, false) => "",
        };
        let _ = writeln!(out, "syntax.{key} {}{flags}", style.color.to_hex());
    }
    for info in METRIC_TOKENS {
        let value = t.metrics.get(info.name).expect("every listed metric has a value");
        let _ = writeln!(out, "{} {value}", info.name);
    }
    for (label, look) in LOOKS {
        let c = t.translucent(look).expect("80% is translucent");
        for (name, color) in [
            ("text.primary", c.text),
            ("text.muted", c.muted),
            ("text.faint", c.faint),
            ("git.branch", c.branch),
            ("sidebar.row.selected.text", c.selected_text),
            ("sidebar.row.selected.muted", c.selected_muted),
        ] {
            let _ = writeln!(out, "translucent {label} {name} {}", color.to_hex());
        }
    }
    let contrast = t.high_contrast();
    for ((name, plain), (_, color)) in t.ui_tokens().zip(contrast.ui_tokens()) {
        if color.to_hex() != plain.to_hex() {
            let _ = writeln!(out, "increase_contrast {name} {}", color.to_hex());
        }
    }
    for info in METRIC_TOKENS {
        let (plain, value) = (t.metrics.get(info.name), contrast.metrics.get(info.name));
        if let Some(value) = value.filter(|v| Some(*v) != plain) {
            let _ = writeln!(out, "increase_contrast {} {value}", info.name);
        }
    }
    out
}

/// Lines only one side has, marked like a unified diff (order kept), so a failure says
/// which tokens moved without an external diff tool.
fn changed_lines(want: &str, got: &str) -> String {
    let (want_set, got_set): (BTreeSet<&str>, BTreeSet<&str>) =
        (want.lines().collect(), got.lines().collect());
    let mut out = String::new();
    for line in want.lines().filter(|l| !got_set.contains(l)) {
        let _ = writeln!(out, "- {line}");
    }
    for line in got.lines().filter(|l| !want_set.contains(l)) {
        let _ = writeln!(out, "+ {line}");
    }
    out
}

#[test]
fn built_in_theme_tokens_match_the_golden_files() {
    let dir = golden_dir();
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    if update {
        std::fs::create_dir_all(&dir).expect("create the golden directory");
    }
    let mut failures = Vec::new();
    let mut expected_files = BTreeSet::new();
    for theme in builtins() {
        let name = file_name(&theme.name);
        let path = dir.join(&name);
        let got = render(theme);
        expected_files.insert(name.clone());
        if update {
            std::fs::write(&path, &got).expect("write a golden file");
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(want) if want == got => {}
            Ok(want) => failures.push(format!("{name}:\n{}", changed_lines(&want, &got))),
            Err(e) => failures.push(format!("{name}: {e}")),
        }
    }
    // A theme that was renamed or removed must not leave a stale snapshot behind.
    let entries = std::fs::read_dir(&dir).expect("read the golden directory");
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !expected_files.contains(&name) {
            if update {
                std::fs::remove_file(entry.path()).expect("remove a stale golden file");
            } else {
                failures.push(format!("{name}: no built-in theme has this name any more"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "token snapshots differ (UPDATE_GOLDEN=1 cargo test -p wormhole-core --test golden_tokens \
         rewrites them; review the diff):\n\n{}",
        failures.join("\n")
    );
}

#[test]
fn file_names_are_unique_slugs() {
    let names: BTreeSet<_> = builtins().iter().map(|t| file_name(&t.name)).collect();
    assert_eq!(names.len(), builtins().len());
    assert_eq!(file_name("Catppuccin Latte"), "catppuccin-latte.txt");
}
