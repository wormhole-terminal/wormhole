//! Source lints for the design language (`src/style/`). They read the sources as
//! text, so they live here rather than in `style`: a unit test inside `src` would
//! have to be excluded from its own scan (it names every pattern it forbids), and
//! this one needs neither GPUI nor a display to run.

use std::path::{Path, PathBuf};

/// Spacing helpers off the 4-point grid. A view that needs one of these
/// wants a shared component in `style/` instead.
const OFF_GRID: [&str; 10] =
    ["_0p5(", "_1p5(", "_2p5(", "_3p5(", "gap_5(", "p_5(", "px_5(", "py_5(", "pl_5(", "pr_5("];

/// GPUI's fixed corner steps; radii come from the theme's metrics instead.
const FIXED_RADII: [&str; 4] = ["rounded_sm()", "rounded_md()", "rounded_lg()", "rounded_xl()"];

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

fn all_sources() -> Vec<PathBuf> {
    let mut files = Vec::new();
    sources(&src(), &mut files);
    files.sort();
    files
}

/// The lines of every source file that contain one of `patterns` in code (not in a comment).
fn offenders(patterns: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for file in all_sources() {
        let text = std::fs::read_to_string(&file).expect("read source");
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if let Some(bad) = patterns.iter().find(|p| code.contains(*p)) {
                found.push(format!("{}:{}: {}", file.display(), n + 1, bad.trim_end_matches('(')));
            }
        }
    }
    found
}

/// GPUI's fixed border widths and text sizes, and a corner given as a bare number: the
/// theme's metrics (`ui.border_width`, `ui.radius*`) and the type roles replace them. A
/// miniature that draws at its own scale (the theme cards) names its numbers instead
/// (`PREVIEW_*`), which passes.
const RAW_SHAPES: [&str; 8] = [
    "border_1(",
    "border_2(",
    "border_b_1(",
    "border_t_1(",
    "border_l_1(",
    "border_r_1(",
    "text_xs(",
    "text_sm(",
];

/// Files that still draw with them and are being moved onto the metrics. (`style/` is
/// where the type roles are defined, in terms of GPUI's sizes.)
const RAW_SHAPE_EXEMPT: [&str; 1] = ["/style/"];

#[test]
fn borders_corners_and_text_sizes_come_from_the_theme() {
    let mut found = Vec::new();
    for file in all_sources() {
        let path = file.to_string_lossy().replace('\\', "/");
        if RAW_SHAPE_EXEMPT.iter().any(|exempt| path.contains(exempt)) {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("read source");
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            let raw_corner = code.match_indices("rounded(px(").any(|(at, pattern)| {
                code[at + pattern.len()..].starts_with(|c: char| c.is_ascii_digit() || c == '-')
            });
            if raw_corner || RAW_SHAPES.iter().any(|p| code.contains(p)) {
                found.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "use ui.border_width, ui.radius* or a TextRole (or name the number):\n{}",
        found.join("\n")
    );
}

#[test]
fn spacing_stays_on_the_grid() {
    let found = offenders(&OFF_GRID);
    assert!(found.is_empty(), "off-grid spacing:\n{}", found.join("\n"));
}

/// Corner radii come from the theme's metrics, not from GPUI's fixed steps.
#[test]
fn radii_come_from_the_theme() {
    let found = offenders(&FIXED_RADII);
    assert!(found.is_empty(), "fixed radius, use ui.radius*:\n{}", found.join("\n"));
}

/// A top-level function of the design language: its name, whether the line above it
/// carries `#[track_caller]`, and its body with string literals blanked out.
struct Builder {
    name: String,
    line: usize,
    track_caller: bool,
    body: String,
}

/// Blanks the contents of string literals so braces and `div()` inside them don't count.
fn blank_strings(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let (mut in_str, mut escaped) = (false, false);
    for c in code.chars() {
        if in_str {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => {
                    in_str = false;
                    out.push('"');
                }
                _ => {}
            }
        } else {
            if c == '"' {
                in_str = true;
            }
            out.push(c);
        }
    }
    out
}

/// The free functions (column 0) of one file with their bodies.
fn builders(text: &str) -> Vec<Builder> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let sig = line.strip_prefix("pub(crate) ").or(line.strip_prefix("pub(super) ")).unwrap_or(line);
        let sig = sig.strip_prefix("pub ").unwrap_or(sig);
        let Some(rest) = sig.strip_prefix("fn ") else {
            i += 1;
            continue;
        };
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        let track_caller = lines[..i]
            .iter()
            .rev()
            .take_while(|l| l.trim_start().starts_with("#[") || l.trim_start().starts_with("///"))
            .any(|l| l.trim() == "#[track_caller]");
        // The body is everything from the first `{` after the signature to its match.
        let (mut depth, mut started, mut body) = (0i32, false, String::new());
        let mut j = i;
        'body: while j < lines.len() {
            let code = blank_strings(lines[j].split("//").next().unwrap_or(""));
            for c in code.chars() {
                match c {
                    '{' => {
                        depth += 1;
                        started = true;
                    }
                    '}' => depth -= 1,
                    _ => {}
                }
                if started {
                    body.push(c);
                }
                if started && depth == 0 {
                    break 'body;
                }
            }
            body.push('\n');
            j += 1;
        }
        found.push(Builder { name, line: i + 1, track_caller, body });
        i = j + 1;
    }
    found
}

/// The helpers that build an element are `#[track_caller]`, so the inspector names the
/// view that used them rather than a line in `style/`. One that creates a `div()` without
/// the attribute would report its own file for every use.
#[test]
fn element_builders_are_track_caller() {
    let src = src();
    let mut files: Vec<PathBuf> =
        all_sources().into_iter().filter(|f| f.starts_with(src.join("style"))).collect();
    files.push(src.join("select.rs"));
    let mut missing = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read source");
        for b in builders(&text) {
            if b.body.contains("div()") && !b.track_caller {
                missing.push(format!("{}:{}: fn {}", file.display(), b.line, b.name));
            }
        }
    }
    assert!(missing.is_empty(), "element builders need #[track_caller]:\n{}", missing.join("\n"));
}

#[test]
fn builder_scan_reads_attributes_and_bodies() {
    let text = "\
/// Doc.
#[track_caller]
pub fn ok(ui: u8) -> Div {
    div().child(\"}\")
}

pub fn bad() -> Div {
    div()
}

fn plain() -> u8 {
    1
}
";
    let found = builders(text);
    let summary: Vec<_> =
        found.iter().map(|b| (b.name.as_str(), b.track_caller, b.body.contains("div()"))).collect();
    assert_eq!(summary, [("ok", true, true), ("bad", false, true), ("plain", false, false)]);
}

/// Every shared component of the design language is in the Theme Studio's gallery, so a
/// theme can be checked against all of them. A new `pub fn` in `style/` needs a
/// specimen in `studio/gallery/` that calls it as `style::name(`.
#[test]
fn the_gallery_shows_every_style_component() {
    // Nothing is exempt: every component has a specimen of its own.
    const EXEMPT: &[&str] = &[];
    let read_dir = |dir: PathBuf| -> String {
        let mut files = Vec::new();
        sources(&dir, &mut files);
        files.sort();
        files.iter().map(|f| std::fs::read_to_string(f).expect("read source")).collect()
    };
    let style = read_dir(src().join("style"));
    let gallery = read_dir(src().join("workspace/studio/gallery"));
    let missing: Vec<String> = public_fns(&style)
        .into_iter()
        .filter(|name| !EXEMPT.contains(&name.as_str()))
        .filter(|name| !gallery.contains(&format!("style::{name}(")))
        .collect();
    assert!(missing.is_empty(), "not in the Studio gallery: {}", missing.join(", "));
}

/// The names of the free `pub fn`s (at column 0) in some source text.
fn public_fns(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.strip_prefix("pub fn "))
        .map(|rest| rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect())
        .collect()
}

#[test]
fn public_fns_reads_free_functions_only() {
    let text =
        "pub fn row(x: u8) {}\n    pub fn method() {}\npub(crate) fn hidden() {}\npub fn icon<T>() {}\n";
    assert_eq!(public_fns(text), ["row", "icon"]);
}

// ---------------------------------------------------------------- terminal grid painting

/// Painting calls that each put a primitive in the scene. Outside a layer every one of them goes
/// into GPUI's bounds tree to find its z-order; a screen of colored cells is thousands of them, and
/// that tree once took nine tenths of the main thread.
const PER_CELL_PAINTS: [&str; 6] = [
    "paint_quad(",
    "paint_glyph(",
    "paint_emoji(",
    "paint_path(",
    "paint_strikethrough(",
    "paint_underlines(",
];

/// The lines (1-based) of `code` that paint inside a loop over the grid's rows (`in cache.rows()`)
/// with no `paint_layer(` around them. Braces are counted with strings blanked and comments cut.
fn grid_paints_outside_layers(code: &str) -> Vec<usize> {
    #[derive(PartialEq)]
    enum Open {
        Layer,
        Rows,
        Other,
    }
    let code: String = blank_strings(code)
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let mut stack: Vec<Open> = Vec::new();
    let (mut next, mut line, mut found) = (Open::Other, 1, Vec::new());
    let mut rest = code.as_str();
    while let Some(c) = rest.chars().next() {
        if rest.starts_with("paint_layer(") {
            next = Open::Layer;
        } else if rest.starts_with("in cache.rows()") {
            next = Open::Rows;
        } else if PER_CELL_PAINTS.iter().any(|p| rest.starts_with(p))
            && stack.contains(&Open::Rows)
            && !stack.contains(&Open::Layer)
        {
            found.push(line);
        }
        match c {
            '{' => stack.push(std::mem::replace(&mut next, Open::Other)),
            '}' => {
                stack.pop();
            }
            '\n' => line += 1,
            _ => {}
        }
        rest = &rest[c.len_utf8()..];
    }
    found.dedup();
    found
}

#[test]
fn the_terminal_paints_its_cells_in_layers() {
    let mut files = Vec::new();
    sources(&src().join("terminal_view/render"), &mut files);
    let mut found = Vec::new();
    for file in files {
        let text = read(&file);
        for line in grid_paints_outside_layers(&text) {
            found.push(format!("{}:{line}", file.display()));
        }
    }
    assert!(found.is_empty(), "paint the grid's cells inside `window.paint_layer(…)`:\n{}", found.join("\n"));
}

#[test]
fn grid_paint_scan_finds_paints_outside_layers_only() {
    let code = r#"
        for (row, painted) in cache.rows() {
            window.paint_quad(fill(a, b)); // outside a layer: line 3
        }
        window.paint_layer(grid, |window| {
            for (row, painted) in cache.rows() {
                window.paint_glyph(o, f, g, s, c);
                let s = "}"; // a brace in a string doesn't count
                window.paint_quad(fill(a, b));
            }
        });
        window.paint_quad(fill(cursor, color)); // one quad, not per cell
        for (_, painted) in cache.rows() {
            for span in &painted.underlines {
                paint_underlines(&u, c, window);
            }
        }
    "#;
    assert_eq!(grid_paints_outside_layers(code), [3, 15]);
}

// ---------------------------------------------------------------- devctl docs

/// Words in backticks in the devctl skill that aren't devctl commands.
const NOT_DEVCTL_COMMANDS: &[&str] =
    &["devctl", "pkill", "killall", "rm", "kill", "git", "codex", "claude", "cmd2", "codesign"];

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

/// The commands `command()` in `devctl/mod.rs` knows: its `"name" =>` arms.
fn devctl_commands(source: &str) -> Vec<String> {
    let body = source.split("fn command(").nth(1).expect("devctl/mod.rs has fn command(");
    let body = body.split("_ => return None").next().unwrap_or(body);
    body.lines()
        .filter_map(|line| {
            line.trim().strip_prefix('"')?.split_once("\" =>").map(|(name, _)| name.to_string())
        })
        .collect()
}

/// The commands the module header lists. A line `//!   usage   description` starts one or more
/// commands (`a <x> | b | c [y]`, the usage ending at two spaces); a description goes on at a
/// deeper indent.
fn header_commands(source: &str) -> Vec<String> {
    source
        .lines()
        .take_while(|line| line.starts_with("//!"))
        .filter_map(|line| line.strip_prefix("//!   "))
        .filter(|rest| !rest.starts_with(' '))
        .flat_map(|rest| {
            let usage = rest.split("  ").next().unwrap_or(rest);
            usage
                .split(" | ")
                .filter_map(|form| form.split_whitespace().next())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|name| name != "|")
        .collect()
}

/// The commands a markdown text names: inline code that starts with a command-like word, and the
/// lines a code block `printf`s into a control file (`printf 'a\nb\n' > "$S/ctl/cmd"`).
fn markdown_commands(text: &str) -> Vec<String> {
    let looks_like_command = |word: &str| {
        word.starts_with(|c: char| c.is_ascii_lowercase())
            && word.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    let mut found = Vec::new();
    let mut in_code_block = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            let Some(script) = line.trim_start().strip_prefix("printf '") else { continue };
            let Some((script, target)) = script.split_once('\'') else { continue };
            if target.contains("/cmd") {
                found.extend(
                    script.split("\\n").filter_map(|l| l.split_whitespace().next()).map(str::to_string),
                );
            }
            continue;
        }
        for code in line.split('`').skip(1).step_by(2) {
            if let Some(word) = code.split_whitespace().next().filter(|w| looks_like_command(w)) {
                found.push(word.to_string());
            }
        }
    }
    found.retain(|word| !NOT_DEVCTL_COMMANDS.contains(&word.as_str()));
    found
}

/// The devctl docs (the header of `devctl/mod.rs` and `.claude/skills/devctl/SKILL.md`) name only
/// commands that exist, so a renamed or removed command can't leave instructions behind that fail.
#[test]
fn the_devctl_docs_name_only_commands_that_exist() {
    let source = read(&src().join("devctl/mod.rs"));
    let known = devctl_commands(&source);
    assert!(known.len() > 40, "found only {} commands in command(): did its shape change?", known.len());
    let skill = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.claude/skills/devctl/SKILL.md"));
    for (doc, named) in
        [("devctl/mod.rs header", header_commands(&source)), ("devctl skill", markdown_commands(&skill))]
    {
        assert!(named.len() > 10, "found only {} commands in the {doc}: did its shape change?", named.len());
        let mut unknown: Vec<&String> = named.iter().filter(|name| !known.contains(name)).collect();
        unknown.dedup();
        assert!(
            unknown.is_empty(),
            "the {doc} names commands that command() doesn't have: {unknown:?}\n(not a command? add it to \
             NOT_DEVCTL_COMMANDS; a header line starting at `//!   ` is a command line, a description goes on \
             deeper)"
        );
    }
}

#[test]
fn devctl_doc_scans_find_commands_and_skip_the_rest() {
    let source = "//! Intro `not this`.\n//!\n//!   shot <name>     render\n//!                   on more lines\n\
                  //!   new | tab-close [n] |\n//!   sync <t>  done\n\nfn command(name: &str) {\n    \"shot\" => a,\n    \
                  #[cfg(x)]\n    \"new\" => b,\n    _ => return None,\n    \"after\" => c,\n}\n";
    assert_eq!(devctl_commands(source), ["shot", "new"]);
    assert_eq!(header_commands(source), ["shot", "new", "tab-close", "sync"]);
    let skill = "Use `shot <name>`, `WORMHOLE_DEVCTL`, `$S/ctl/cmd`, `pkill` and `wait-idle 500`.\n\
                 ```sh\nprintf 'resize 1 2\\nnew\\n' > \"$S/ctl/cmd\"\nprintf 'not\\n' > other\n```\n";
    assert_eq!(markdown_commands(skill), ["shot", "wait-idle", "resize", "new"]);
}

// ---------------------------------------------------------------- wording

/// The repository root (two levels above this crate).
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every text file under `dir` (recursively), by extension: fonts and images are skipped.
fn text_files(dir: &Path, out: &mut Vec<PathBuf>) {
    const TEXT: [&str; 9] = ["rs", "md", "json", "jsonc", "toml", "txt", "svg", "sh", "py"];
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            text_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()).is_some_and(|e| TEXT.contains(&e)) {
            out.push(path);
        }
    }
}

/// The files of one directory, not its subdirectories.
fn files_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect()
}

/// Third-party names that happen to contain the word: an agent's package and executable.
const CHAT_ALLOWED: [&str; 2] = ["aider-chat", "kiro-cli-chat"];

/// A sidebar item is a `Session` in code and a "terminal" in the UI and docs; the old word is
/// gone everywhere: identifiers, strings, comments, assets and the public text files (this file is
/// not scanned).
#[test]
fn no_chat_wording() {
    let root = repo();
    let mut files = Vec::new();
    for dir in ["crates/wormhole/src", "crates/wormhole-core/src", "crates/wormhole-core/assets"] {
        text_files(&root.join(dir), &mut files);
    }
    // The repo's own skills only: anything else in .claude/skills is a contributor's personal,
    // untracked skill and not ours to lint.
    let skills = ["devctl", "gpui", "verify"].map(|s| format!(".claude/skills/{s}"));
    for dir in skills.iter().map(String::as_str).chain([".claude/agents", ".claude/rules", "tools/visual"]) {
        text_files(&root.join(dir), &mut files);
    }
    files.extend(["README.md", "CONTRIBUTING.md", "SECURITY.md"].map(|f| root.join(f)));
    files.extend(files_in(&root.join("scripts")));
    assert!(files.len() > 200, "found only {} files: did the layout change?", files.len());
    let word = ["c", "hat"].concat();
    let mut found = Vec::new();
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        for (n, line) in text.lines().enumerate() {
            let mut line = line.to_lowercase();
            for allowed in CHAT_ALLOWED {
                line = line.replace(allowed, "");
            }
            if line.contains(&word) {
                found.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "say session (code) or terminal (UI, docs):\n{}", found.join("\n"));
}

/// A string literal in a Rust source, with the line it starts on.
struct Literal {
    line: usize,
    text: String,
    /// It is an argument of a `tracing` macro (a log line, not something on screen).
    logged: bool,
}

/// The string literals of a Rust source (plain and raw), skipping comments and char literals.
fn string_literals(source: &str) -> Vec<Literal> {
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
            found.push(Literal { line: start_line, text, logged });
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

/// Whether a literal's text says "session" to a reader. Element ids, group names and keys (one
/// word of lower case, digits, `-` and `_`) and `format!` placeholders don't.
fn says_session(text: &str) -> bool {
    let is_id = text.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if is_id {
        return false;
    }
    let mut prose = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            c if depth == 0 => prose.push(c),
            _ => {}
        }
    }
    let prose = prose.to_lowercase();
    prose
        .match_indices("session")
        .any(|(at, _)| !prose[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_'))
}

/// Strings in the app that a user may read and still say "session" (each one reviewed).
const SESSION_ALLOWED: [&str; 0] = [];

/// Identifiers say `session`, the UI says "terminal": a heuristic over the app's string literals
/// (log lines, test code, element ids and format placeholders don't count). Dev-only tools
/// (`devctl/`, the inspector) are not scanned.
#[test]
fn ui_says_terminal_not_session() {
    let mut files = Vec::new();
    sources(&src(), &mut files);
    files.sort();
    let mut found = Vec::new();
    for file in files {
        let path = file.to_string_lossy().replace('\\', "/");
        let test_file = path.ends_with("tests.rs") || path.contains("/tests/");
        if test_file || path.contains("/devctl/") || path.contains("/inspector/") {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("read source");
        // The unit tests sit at the end of a file, in `#[cfg(test)] mod tests`.
        let code = text.split("#[cfg(test)]\nmod ").next().unwrap_or(&text);
        for literal in string_literals(code) {
            if !literal.logged
                && says_session(&literal.text)
                && !SESSION_ALLOWED.contains(&literal.text.as_str())
            {
                found.push(format!("{}:{}: {:?}", file.display(), literal.line, literal.text));
            }
        }
    }
    assert!(found.is_empty(), "user-facing text says \"terminal\", not \"session\":\n{}", found.join("\n"));
}

#[test]
fn literal_scan_finds_strings_and_skips_comments_chars_and_logs() {
    let source = "let a = \"one session\"; // \"not this\"\nlet q = '\"';\nlet b: &'static str = r#\"raw \"x\"\"#;\n\
                  /* \"nor this\" */ tracing::warn!(\"logged {x}\", x = \"also logged\");\nlet c = \"multi\\\n line\";\n";
    let found: Vec<_> = string_literals(source).into_iter().map(|l| (l.line, l.text, l.logged)).collect();
    assert_eq!(
        found,
        [
            (1, "one session".to_string(), false),
            (3, "raw \"x\"".to_string(), false),
            (4, "logged {x}".to_string(), true),
            (4, "also logged".to_string(), true),
            (5, "multi\\\n line".to_string(), false),
        ]
    );
    assert!(says_session("Close this session?") && says_session("Sessions"));
    assert!(
        !says_session("new-session") && !says_session("{session}\u{1f}{path}") && !says_session("obsession")
    );
}
