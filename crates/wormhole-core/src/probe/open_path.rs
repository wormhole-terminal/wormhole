//! A file named in the terminal's text (`src/main.rs:12:5`): read as a path with an optional line and
//! column, and found on disk before an editor is asked to open it.
//!
//! The text is a program's output and so hostile. Reading it never touches the disk ([`PathRef::parse`]);
//! finding the file does it through `storage::files::resolve_within`, which resolves every link and
//! refuses a path that ends outside the folder it was given, so a link a repository planted cannot send
//! an editor to a file elsewhere. The editor is asked to open the file and never runs it: that is the
//! app's `open_in`, which takes a [`std::path::Path`], not a shell string.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::probe::events::MAX_PATH_BYTES;
use crate::probe::hidden::is_hidden;
use crate::storage::files;

/// A path as written, with the line and column after it when there are some.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRef {
    pub path: String,
    /// 1-based.
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl PathRef {
    /// `path`, `path:line` or `path:line:col`. `None` for text that cannot be a file to open: empty, longer than
    /// [`MAX_PATH_BYTES`], with a control or invisible character, a link (`://`), or a `..` segment.
    /// A trailing number counts as a line only after a `:`, so a name with a colon in it stays a name.
    pub fn parse(text: &str) -> Option<Self> {
        let number = |part: &str| -> Option<u32> {
            let digits = (1..=7).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_digit());
            digits.then(|| part.parse().ok()).flatten().filter(|n| *n > 0)
        };
        // Up to two trailing `:N`: the line, then the column.
        let (mut path, mut numbers) = (text, Vec::new());
        while numbers.len() < 2 {
            let Some((rest, tail)) = path.rsplit_once(':') else { break };
            let Some(n) = number(tail) else { break };
            numbers.push(n);
            path = rest;
        }
        let (line, column) = match numbers[..] {
            [] => (None, None),
            [line] => (Some(line), None),
            [column, line] => (Some(line), Some(column)),
            _ => (None, None),
        };
        let bad = path.is_empty()
            || path.len() > MAX_PATH_BYTES
            || path.contains("://")
            || path.chars().any(|c| is_hidden(c) || c.is_control())
            || path.split('/').any(|segment| segment == "..");
        (!bad).then(|| Self { path: path.to_string(), line, column })
    }

    /// The path, a line and column as the text it was read from.
    pub fn text(&self) -> String {
        match (self.line, self.column) {
            (Some(line), Some(column)) => format!("{}:{line}:{column}", self.path),
            (Some(line), None) => format!("{}:{line}", self.path),
            _ => self.path.clone(),
        }
    }
}

/// The real path of the regular file `target` names, looked for in each of `roots` in turn (the terminal's
/// folder first, then the project's): a relative path is taken from the root, an absolute one only when it lies
/// inside it. Every link is resolved and the result must stay inside the root it was found under.
///
/// Reads file metadata only, so it may block on a slow volume: call it off the main thread.
pub fn resolve(roots: &[&Path], target: &PathRef) -> io::Result<PathBuf> {
    let written = Path::new(&target.path);
    let mut last = io::Error::new(io::ErrorKind::NotFound, "no folder to look in");
    for root in roots {
        let relative = if written.is_absolute() {
            // Compared as real paths: `/tmp` is a link to `/private/tmp` on a Mac.
            let real_root = fs::canonicalize(root)?;
            match written.strip_prefix(root).or_else(|_| written.strip_prefix(&real_root)) {
                Ok(inside) => inside.to_path_buf(),
                Err(_) => continue,
            }
        } else {
            written.to_path_buf()
        };
        // `./a.rs` is `a.rs`; `resolve_within` takes a plain path.
        let plain: PathBuf = relative.components().filter(|c| !matches!(c, Component::CurDir)).collect();
        let Some(plain) = plain.to_str().filter(|text| !text.is_empty()) else { continue };
        match files::resolve_within(root, plain) {
            Ok(real) => return Ok(real),
            Err(err) => last = err,
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> Option<(String, Option<u32>, Option<u32>)> {
        PathRef::parse(text).map(|p| (p.path, p.line, p.column))
    }

    #[test]
    fn a_path_may_carry_a_line_and_a_column() {
        assert_eq!(parsed("src/a.rs"), Some(("src/a.rs".into(), None, None)));
        assert_eq!(parsed("src/a.rs:12"), Some(("src/a.rs".into(), Some(12), None)));
        assert_eq!(parsed("src/a.rs:12:5"), Some(("src/a.rs".into(), Some(12), Some(5))));
        assert_eq!(parsed("/abs/a.rs:1"), Some(("/abs/a.rs".into(), Some(1), None)));
        assert_eq!(PathRef::parse("src/a.rs:12:5").map(|p| p.text()).as_deref(), Some("src/a.rs:12:5"));
    }

    #[test]
    fn a_number_that_is_not_after_a_colon_stays_in_the_name() {
        assert_eq!(parsed("v1.2"), Some(("v1.2".into(), None, None)));
        assert_eq!(parsed("a:b"), Some(("a:b".into(), None, None)));
        assert_eq!(parsed("a:0"), Some(("a:0".into(), None, None)), "line numbers start at 1");
    }

    #[test]
    fn text_that_cannot_be_a_file_is_refused() {
        for bad in [
            "",
            ":12",
            "../x",
            "a/../b",
            "https://a.io/x",
            "a\u{0}b",
            "a\nb",
            "a\u{202e}b",
            &"a".repeat(MAX_PATH_BYTES + 1),
        ] {
            assert_eq!(PathRef::parse(bad), None, "{bad:?}");
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wh-open-path-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn a_file_inside_the_folder_resolves_and_a_link_out_does_not() {
        let root = scratch("links");
        fs::create_dir_all(root.join("src")).expect("dir");
        fs::write(root.join("src/a.rs"), "x").expect("file");
        let outside = scratch("outside");
        fs::write(outside.join("secret"), "s").expect("file");
        crate::platform::fs::symlink(outside.join("secret"), root.join("planted")).expect("link");
        let real_root = fs::canonicalize(&root).expect("root");
        let find = |text: &str| resolve(&[&root], &PathRef::parse(text).expect("parses"));

        assert_eq!(find("src/a.rs:3").expect("found"), real_root.join("src/a.rs"));
        assert_eq!(find("./src/a.rs").expect("found"), real_root.join("src/a.rs"));
        assert_eq!(
            find(&format!("{}/src/a.rs", root.display())).expect("absolute inside"),
            real_root.join("src/a.rs")
        );
        assert!(find("planted").is_err(), "a link that leaves the folder");
        assert!(find("missing.rs").is_err());
        assert!(find("src").is_err(), "a folder is not a file");
        assert!(find(&format!("{}/secret", outside.display())).is_err(), "an absolute path outside");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn the_second_folder_is_tried_when_the_first_has_no_such_file() {
        let (first, second) = (scratch("first"), scratch("second"));
        fs::write(second.join("b.rs"), "x").expect("file");
        let found = resolve(&[&first, &second], &PathRef::parse("b.rs").expect("parses"));
        assert_eq!(found.expect("found"), fs::canonicalize(&second).expect("root").join("b.rs"));
        let _ = fs::remove_dir_all(&first);
        let _ = fs::remove_dir_all(&second);
    }
}
