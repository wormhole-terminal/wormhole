//! Opening one file in the app the Open in… setting names, at a line where that app can: the
//! rules (which app takes which mechanism, how a path is written into a URL) as pure functions,
//! so they are tested here. The app turns a [`Launch`] into a process (`window::open_in`).
//!
//! Only mechanisms the editors document themselves are used; for every other app the file is
//! simply opened and the line is dropped:
//!
//! - **VS Code and its forks**: `<scheme>://file/<absolute path>:<line>`, a URL the app registers
//!   itself (VS Code's "Opening VS Code with URLs"; Insiders, Cursor, Windsurf and VSCodium keep
//!   the shape with their own scheme).
//! - **Zed**: its bundled command line, `cli <path>:<line>` (`cli --help`: "Use `path:line:column`
//!   syntax to open a file at the given line").
//! - **Sublime Text**: its bundled `subl <path>:<line>`.
//! - **JetBrains IDEs and Android Studio**: the launcher's `--line <n> <path>`, passed the way
//!   JetBrains documents for macOS (`open -n -a <app> --args …`): a new launcher process hands
//!   the file to the IDE that is running, and starts it when it isn't. No Toolbox involved.
//! - **Finder** has no editor to launch: the file is revealed.
//!
//! As for Open in… itself, the person asks an app they chose to open a file, so nothing here
//! runs anything from the project and project trust doesn't come into it. A path that leads out
//! of the repository is refused before this is asked (`fsutil::resolve_within`).

use std::path::{Path, PathBuf};

use crate::platform::open_in::FINDER;

/// How to open a file, before it is turned into a process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Launch {
    /// `open -b <bundle> <path>`: the app opens the file, wherever its cursor was.
    Open { path: PathBuf },
    /// `open -b <bundle> <url>`: the app's own URL, which carries the line.
    Url { url: String },
    /// `open -n -b <bundle> --args <args…>`.
    Args { args: Vec<String> },
    /// `<app>/<relative> <arg>`: a command line the app brings along.
    Bundled { relative: &'static str, arg: String },
    /// `open -R <path>`: Finder shows the file selected.
    Reveal { path: PathBuf },
}

/// What an app takes to open a file at a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Way {
    /// `<scheme>://file/<path>:<line>`.
    Url(&'static str),
    /// `<path>:<line>` to the command line inside the app bundle.
    Bundled(&'static str),
    /// `--line <n> <path>` to the app's launcher.
    LineFlag,
    /// Nothing we know: open the file.
    None,
}

fn way(bundle: &str) -> Way {
    match bundle {
        "com.microsoft.VSCode" => Way::Url("vscode"),
        "com.microsoft.VSCodeInsiders" => Way::Url("vscode-insiders"),
        "com.todesktop.230313mzl4w4u92" => Way::Url("cursor"),
        "com.exafunction.windsurf" => Way::Url("windsurf"),
        "com.vscodium" => Way::Url("vscodium"),
        "dev.zed.Zed" | "dev.zed.Zed-Preview" => Way::Bundled("Contents/MacOS/cli"),
        "com.sublimetext.4" | "com.sublimetext.3" => Way::Bundled("Contents/SharedSupport/bin/subl"),
        b if b.starts_with("com.jetbrains.") || b == "com.google.android.studio" => Way::LineFlag,
        _ => Way::None,
    }
}

/// Whether [`launch`] can put the cursor on a line in this app: the tooltip says "at line N" only
/// then.
pub fn opens_at_line(bundle: &str) -> bool {
    way(bundle) != Way::None
}

/// How to open `path` (absolute, and already checked to be inside the repository) in the app with
/// this bundle identifier, at `line` (1-based) when the app can. `None` for a relative path.
pub fn launch(bundle: &str, path: &Path, line: Option<u32>) -> Option<Launch> {
    if !path.is_absolute() {
        return None;
    }
    if bundle == FINDER.bundle {
        return Some(Launch::Reveal { path: path.to_path_buf() });
    }
    let open = || Launch::Open { path: path.to_path_buf() };
    let (Some(line), Some(text)) = (line.filter(|&line| line > 0), path.to_str()) else {
        return Some(open());
    };
    Some(match way(bundle) {
        Way::Url(scheme) => Launch::Url { url: format!("{scheme}://file{}:{line}", encode_path(text)) },
        // The line is read off the end of the argument, so a colon inside the path would be
        // ambiguous: open the file without the line rather than a different one.
        Way::Bundled(relative) if !text.contains(':') => {
            Launch::Bundled { relative, arg: format!("{text}:{line}") }
        }
        Way::LineFlag => Launch::Args { args: vec!["--line".into(), line.to_string(), text.into()] },
        Way::Bundled(_) | Way::None => open(),
    })
}

/// `path` as the path part of a URL: everything but letters, digits, `-._~` and the `/` between
/// folders as `%XX` of its UTF-8 bytes (a colon too, so the line after it is the only one).
fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len() + 8);
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(char::from(byte))
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The hover icon's tooltip: "Open in Zed at line 12", "Reveal in Finder".
pub fn tooltip(bundle: &str, app: &str, line: Option<u32>) -> String {
    if bundle == FINDER.bundle {
        return "Reveal in Finder".into();
    }
    match line.filter(|&line| line > 0 && opens_at_line(bundle)) {
        Some(line) => format!("Open in {app} at line {line}"),
        None => format!("Open in {app}"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Launch, encode_path, launch, opens_at_line, tooltip};
    use crate::platform::open_in::{EDITORS, FINDER};

    const FILE: &str = "/Users/me/code/app/src/main.rs";

    fn open_at(bundle: &str, path: &str, line: Option<u32>) -> Launch {
        launch(bundle, Path::new(path), line).expect("an absolute path")
    }

    fn plain(path: &str) -> Launch {
        Launch::Open { path: PathBuf::from(path) }
    }

    #[test]
    fn vs_code_and_its_forks_take_their_own_url() {
        for (bundle, scheme) in [
            ("com.microsoft.VSCode", "vscode"),
            ("com.microsoft.VSCodeInsiders", "vscode-insiders"),
            ("com.todesktop.230313mzl4w4u92", "cursor"),
            ("com.exafunction.windsurf", "windsurf"),
            ("com.vscodium", "vscodium"),
        ] {
            assert_eq!(
                open_at(bundle, FILE, Some(42)),
                Launch::Url { url: format!("{scheme}://file{FILE}:42") },
                "{bundle}"
            );
        }
    }

    #[test]
    fn zed_and_sublime_take_the_path_and_line_on_their_bundled_command_line() {
        let bundled = |relative, arg: &str| Launch::Bundled { relative, arg: arg.into() };
        for bundle in ["dev.zed.Zed", "dev.zed.Zed-Preview"] {
            assert_eq!(open_at(bundle, FILE, Some(7)), bundled("Contents/MacOS/cli", &format!("{FILE}:7")));
        }
        for bundle in ["com.sublimetext.4", "com.sublimetext.3"] {
            assert_eq!(
                open_at(bundle, FILE, Some(7)),
                bundled("Contents/SharedSupport/bin/subl", &format!("{FILE}:7"))
            );
        }
    }

    #[test]
    fn jetbrains_ides_take_the_line_flag() {
        for editor in EDITORS.iter().filter(|e| e.bundle.starts_with("com.jetbrains.")) {
            assert_eq!(
                open_at(editor.bundle, FILE, Some(3)),
                Launch::Args { args: ["--line", "3", FILE].map(String::from).into() },
                "{}",
                editor.name
            );
        }
        assert!(matches!(open_at("com.google.android.studio", FILE, Some(3)), Launch::Args { .. }));
    }

    #[test]
    fn an_app_without_a_known_way_opens_the_file_and_drops_the_line() {
        for bundle in ["com.apple.dt.Xcode", "com.panic.Nova", "com.barebones.bbedit", "com.example.Unknown"]
        {
            assert_eq!(open_at(bundle, FILE, Some(9)), plain(FILE), "{bundle}");
            assert!(!opens_at_line(bundle));
        }
    }

    #[test]
    fn without_a_line_every_app_just_opens_the_file() {
        for editor in EDITORS {
            assert_eq!(open_at(editor.bundle, FILE, None), plain(FILE), "{}", editor.name);
            // A line of 0 is no line (diff rows without a number on the new side).
            assert_eq!(open_at(editor.bundle, FILE, Some(0)), plain(FILE), "{}", editor.name);
        }
    }

    #[test]
    fn finder_reveals_the_file_whatever_the_line() {
        let reveal = Launch::Reveal { path: PathBuf::from(FILE) };
        assert_eq!(open_at(FINDER.bundle, FILE, None), reveal);
        assert_eq!(open_at(FINDER.bundle, FILE, Some(5)), reveal);
    }

    #[test]
    fn a_relative_path_is_refused() {
        assert_eq!(launch("dev.zed.Zed", Path::new("src/main.rs"), Some(1)), None);
        assert_eq!(launch(FINDER.bundle, Path::new("../x"), None), None);
    }

    #[test]
    fn a_path_is_written_into_a_url_byte_by_byte() {
        assert_eq!(encode_path("/a/b-c_d.e~f/Z9"), "/a/b-c_d.e~f/Z9");
        assert_eq!(encode_path("/My Code/a#b?c%d&e.rs"), "/My%20Code/a%23b%3Fc%25d%26e.rs");
        // A colon would be read as the line's; non-ASCII goes as UTF-8 bytes.
        assert_eq!(encode_path("/a:12/ä.rs"), "/a%3A12/%C3%A4.rs");
        assert_eq!(
            open_at("com.microsoft.VSCode", "/tmp/My Proj/a b#1.rs", Some(2)),
            Launch::Url { url: "vscode://file/tmp/My%20Proj/a%20b%231.rs:2".into() }
        );
    }

    #[test]
    fn a_colon_in_the_path_keeps_the_command_line_apps_from_guessing_the_line() {
        let path = "/tmp/a:12/main.rs";
        assert_eq!(open_at("dev.zed.Zed", path, Some(3)), plain(path));
        assert_eq!(open_at("com.sublimetext.4", path, Some(3)), plain(path));
        // The URL and the flag carry the line apart from the path.
        assert!(
            matches!(open_at("com.microsoft.VSCode", path, Some(3)), Launch::Url { url } if url.ends_with("a%3A12/main.rs:3"))
        );
        assert!(matches!(open_at("com.jetbrains.PhpStorm", path, Some(3)), Launch::Args { .. }));
    }

    #[test]
    fn the_tooltip_names_the_app_and_the_line_only_where_it_is_used() {
        assert_eq!(tooltip("dev.zed.Zed", "Zed", Some(12)), "Open in Zed at line 12");
        assert_eq!(tooltip("dev.zed.Zed", "Zed", None), "Open in Zed");
        assert_eq!(tooltip("com.panic.Nova", "Nova", Some(12)), "Open in Nova");
        assert_eq!(tooltip(FINDER.bundle, "Finder", Some(12)), "Reveal in Finder");
    }
}
