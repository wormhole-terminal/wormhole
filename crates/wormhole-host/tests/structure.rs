//! The host's structure rules, checked on every test run as for the other crates
//! (`wormhole-vt/tests/structure.rs`): files stay small; the host links only what it must
//! (`session-host.md`, section 5: no GPUI, SQLite, TLS or the app's core); the session, the part
//! that decides, does no I/O and reads no clock, so it is tested on a virtual one; and `unsafe`
//! stays in the two files that talk to the system.

use std::fs;
use std::path::{Path, PathBuf};

/// A tripwire, not the target (files aim for ~600 lines).
const MAX_LINES: usize = 1000;

/// What `[dependencies]` may name.
const ALLOWED_DEPENDENCIES: &[&str] = &[
    "wormhole-term",
    "wormhole-wire",
    "anyhow",
    "libc",
    "tracing",
    "polling",
    "security-framework",
    "security-framework-sys",
    "core-foundation",
];

/// What the session's code must not name: sockets, files, the poller, the system, the clock.
const SESSION_FORBIDS: &[&str] = &[
    "std::os::unix::net",
    "std::fs",
    "polling::",
    "libc::",
    "Instant::now",
    "SystemTime::now",
    "std::thread",
];

/// The files that may hold `unsafe` (each in a small function with `#[allow(unsafe_code)]`).
const UNSAFE_IN: &[&str] = &["src/sys.rs", "src/serve.rs", "src/peer.rs"];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn relative(path: &Path) -> String {
    path.strip_prefix(crate_dir()).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// The file's code lines: without `//` comments (doc comments included).
fn code(path: &Path) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .map(|line| match line.find("//") {
            Some(at) if !line[..at].contains('"') => line[..at].to_owned(),
            _ => line.to_owned(),
        })
        .collect()
}

#[test]
fn files_stay_under_the_length_limit() {
    let mut files = Vec::new();
    for dir in ["src", "tests"] {
        rust_files(&crate_dir().join(dir), &mut files);
    }
    let long: Vec<String> = files
        .iter()
        .filter_map(|path| {
            let lines = fs::read_to_string(path).ok()?.lines().count();
            (lines > MAX_LINES).then(|| format!("{} ({lines} lines)", relative(path)))
        })
        .collect();
    assert!(long.is_empty(), "split these by responsibility: {long:#?}");
}

#[test]
fn the_host_links_only_what_it_needs() {
    let manifest = fs::read_to_string(crate_dir().join("Cargo.toml")).expect("the manifest");
    let mut in_dependencies = false;
    let mut extra = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            // The platform tables too (`[target.'cfg(..)'.dependencies]`), not the dev-dependencies.
            in_dependencies = line == "[dependencies]"
                || (line.starts_with("[target.") && line.ends_with(".dependencies]"));
            continue;
        }
        if !in_dependencies || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let name = line.split(['=', '.', ' ']).next().unwrap_or_default();
        if !ALLOWED_DEPENDENCIES.contains(&name) {
            extra.push(name.to_owned());
        }
    }
    assert!(extra.is_empty(), "the host stays small; these are not on its list: {extra:?}");
}

#[test]
fn the_session_does_no_io_and_reads_no_clock() {
    let mut files = Vec::new();
    rust_files(&crate_dir().join("src/session"), &mut files);
    assert!(!files.is_empty());
    let mut found = Vec::new();
    for path in files.iter().filter(|p| !p.ends_with("tests.rs")) {
        for (n, line) in code(path).iter().enumerate() {
            for forbidden in SESSION_FORBIDS {
                if line.contains(forbidden) {
                    found.push(format!("{}:{}: {forbidden}", relative(path), n + 1));
                }
            }
        }
    }
    assert!(found.is_empty(), "the session takes its I/O and its time from the driver: {found:#?}");
}

#[test]
fn unsafe_stays_where_the_system_is() {
    let mut files = Vec::new();
    rust_files(&crate_dir().join("src"), &mut files);
    let found: Vec<String> = files
        .iter()
        .filter(|path| !UNSAFE_IN.contains(&relative(path).as_str()))
        .filter(|path| code(path).iter().any(|line| line.contains("unsafe ") || line.contains("unsafe{")))
        .map(|path| relative(path))
        .collect();
    assert!(found.is_empty(), "unsafe outside {UNSAFE_IN:?}: {found:?}");
}
