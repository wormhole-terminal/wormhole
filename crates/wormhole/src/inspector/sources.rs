//! Where an element was built: its file and line, the source line itself, and its id
//! path without GPUI's own wrappers.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::*;

thread_local! {
    /// Source files by path, split into lines, for showing the line that built an element.
    static SOURCES: RefCell<HashMap<&'static str, Option<Rc<Vec<String>>>>> = RefCell::new(HashMap::new());
}

/// The element's id path without GPUI's numbered views and GPUI Kit's window
/// wrappers: `session-list.session-2`, `change-0-main.rs`.
/// Ids can contain dots themselves (file names), so the path isn't split further.
pub(super) fn element_name(id: &InspectorElementId) -> String {
    clean_path(&id.path.global_id.to_string(), id.instance_id)
}

fn clean_path(path: &str, instance_id: usize) -> String {
    let parts: Vec<&str> = path
        .split('.')
        .filter(|p| !(p.starts_with("view-") && p[5..].chars().all(|c| c.is_ascii_digit())))
        .map(|p| p.rsplit("::").next().unwrap_or(p))
        .skip_while(|p| matches!(*p, "WindowBorder" | "root"))
        .collect();
    let mut name = parts.join(".");
    if instance_id > 0 {
        name.push_str(&format!("  (#{})", instance_id + 1));
    }
    name
}

/// Our files relative to the app's `src/`; a dependency's as `crate: path`.
pub(super) fn short_file(file: &str) -> String {
    if let Some(rest) = file.strip_prefix("crates/wormhole/src/") {
        return rest.into();
    }
    match file.split_once("/registry/src/").and_then(|(_, rest)| rest.split_once('/')) {
        Some((_, dependency)) => dependency.replacen("/src/", ": ", 1),
        None => file.into(),
    }
}

/// The trimmed source line at `file:line`, read from the checkout the app was built from.
pub(super) fn source_line(file: &'static str, line: u32) -> Option<String> {
    let lines = SOURCES.with(|cache| {
        cache
            .borrow_mut()
            .entry(file)
            .or_insert_with(|| {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(file);
                let text = std::fs::read_to_string(path).ok()?;
                Some(Rc::new(text.lines().map(str::to_string).collect()))
            })
            .clone()
    })?;
    let text = lines.get(line.checked_sub(1)? as usize)?.trim();
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::{clean_path, short_file, source_line};

    #[test]
    fn our_files_are_shown_relative_to_src() {
        assert_eq!(short_file("crates/wormhole/src/style/controls.rs"), "style/controls.rs");
        assert_eq!(
            short_file("/home/u/.cargo/registry/src/index.crates.io-6f17d22bba15001f/gpui-0.2.2/src/div.rs"),
            "gpui-0.2.2: div.rs"
        );
        assert_eq!(short_file("elsewhere/main.rs"), "elsewhere/main.rs");
    }

    #[test]
    fn element_paths_lose_views_and_window_wrappers() {
        assert_eq!(
            clean_path("root.WindowBorder.view-12.session-list.session-2", 0),
            "session-list.session-2"
        );
        assert_eq!(clean_path("a::b::root.change-0-main.rs", 0), "change-0-main.rs");
        assert_eq!(clean_path("session-list", 2), "session-list  (#3)");
        // `view-` followed by something that isn't a number is an id like any other.
        assert_eq!(clean_path("view-model.row", 0), "view-model.row");
    }

    #[test]
    fn the_source_line_is_trimmed_and_blank_lines_have_none() {
        let file = "crates/wormhole/src/inspector/sources.rs";
        assert_eq!(
            source_line(file, 1).as_deref(),
            Some("//! Where an element was built: its file and line, the source line itself, and its id")
        );
        assert_eq!(source_line(file, 0), None);
        assert_eq!(source_line(file, 100_000), None);
        assert_eq!(source_line("crates/wormhole/src/no/such/file.rs", 1), None);
    }
}
