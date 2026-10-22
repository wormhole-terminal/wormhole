use std::fs;
use std::path::{Path, PathBuf};

use std::sync::Arc;

use super::load::PARSES;
use super::{
    Appearance, BUILTIN_PREFIX, IconSource, IconTheme, RenderMode, builtin_asset, builtin_family,
    builtin_icons, discover, load_family, pick_shared, pick_theme, theme_named,
};

/// The example theme from Zed's icon theme docs, laid out like a Zed
/// extension (`icon_themes/*.json` + `icons/`).
const ZED_FIXTURE: &str = r#"{
  "$schema": "https://zed.dev/schema/icon_themes/v0.3.0.json",
  "name": "My Icon Theme",
  "author": "Your Name",
  "themes": [
    {
      "name": "My Icon Theme",
      "appearance": "dark",
      "directory_icons": {
        "collapsed": "./icons/folder.svg",
        "expanded": "./icons/folder-open.svg"
      },
      "named_directory_icons": {
        "stylesheets": {
          "collapsed": "./icons/folder-stylesheets.svg",
          "expanded": "./icons/folder-stylesheets-open.svg"
        }
      },
      "chevron_icons": {
        "collapsed": "./icons/chevron-right.svg",
        "expanded": "./icons/chevron-down.svg"
      },
      "file_stems": { "Makefile": "make" },
      "file_suffixes": { "mp3": "audio", "rs": "rust" },
      "file_icons": {
        "audio": { "path": "./icons/audio.svg" },
        "default": { "path": "./icons/file.svg" },
        "make": { "path": "./icons/make.svg" },
        "rust": { "path": "./icons/rust.svg" }
        // Zed's own files don't have comments, but ours may.
      },
      "some_future_field": true
    }
  ]
}"#;

fn zed_extension() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ext = tmp.path().join("my-icons");
    fs::create_dir_all(ext.join("icon_themes")).expect("mkdir");
    fs::create_dir_all(ext.join("icons")).expect("mkdir");
    fs::write(ext.join("icon_themes/my-icon-theme.json"), ZED_FIXTURE).expect("write");
    (tmp, ext)
}

fn file(root: &Path, rel: &str) -> IconSource {
    IconSource::File(root.canonicalize().expect("canonicalize").join(rel))
}

fn builtin_dark() -> &'static IconTheme {
    pick_theme(builtin_family(), false).expect("dark theme")
}

fn key(icon: Option<&IconSource>) -> &'static str {
    match icon {
        Some(IconSource::Builtin(k)) => k.strip_prefix(BUILTIN_PREFIX).unwrap_or(k),
        other => panic!("expected a builtin icon, got {other:?}"),
    }
}

#[test]
fn parses_zed_extension_layout() {
    let (_tmp, ext) = zed_extension();
    let family = load_family(&ext).expect("load");
    assert_eq!(family.name, "My Icon Theme");
    assert_eq!(family.author, "Your Name");
    assert_eq!(family.render, RenderMode::Color);
    let theme = &family.themes[0];
    assert_eq!(theme.appearance, Appearance::Dark);
    // Paths resolve against the extension root, as in Zed.
    assert_eq!(theme.file_icon("src/main.rs"), Some(&file(&ext, "icons/rust.svg")));
    assert_eq!(theme.file_icon("Makefile"), Some(&file(&ext, "icons/make.svg")));
    assert_eq!(theme.file_icon("song.mp3"), Some(&file(&ext, "icons/audio.svg")));
    assert_eq!(theme.file_icon("notes.unknown"), Some(&file(&ext, "icons/file.svg")));
    assert_eq!(
        theme.directory_icon("stylesheets", true),
        Some(&file(&ext, "icons/folder-stylesheets-open.svg"))
    );
    assert_eq!(theme.directory_icon("other", false), Some(&file(&ext, "icons/folder.svg")));
    assert_eq!(theme.chevron(false), Some(&file(&ext, "icons/chevron-right.svg")));
    // Loading the JSON directly gives the same result.
    let direct = load_family(&ext.join("icon_themes/my-icon-theme.json")).expect("load json");
    assert_eq!(direct.themes[0].file_icon("a.rs"), theme.file_icon("a.rs"));
}

#[test]
fn user_theme_inherits_default_associations() {
    // A theme that only defines `file_icons` (no stems/suffixes) still matches
    // files through the bundled theme's associations, as Zed does with its own.
    let tmp = tempfile::tempdir().expect("tempdir");
    let json = tmp.path().join("t.json");
    fs::write(
        &json,
        r#"{ "name": "Mini", "author": "x", "themes": [ { "name": "Mini", "appearance": "light",
             "file_icons": { "docker": { "path": "docker.svg" }, "rust": { "path": "rs.svg" } } } ] }"#,
    )
    .expect("write");
    let family = load_family(&json).expect("load");
    let t = &family.themes[0];
    assert_eq!(t.file_icon("Dockerfile"), Some(&file(tmp.path(), "docker.svg")));
    assert_eq!(t.file_icon("lib.rs"), Some(&file(tmp.path(), "rs.svg")));
    // No "default" icon and no match: nothing.
    assert_eq!(t.file_icon("x.py"), None);
}

#[test]
fn precedence_full_name_then_longest_suffix() {
    let t = builtin_dark();
    assert_eq!(key(t.file_icon("index.d.ts")), "dark/typescript-def.svg");
    assert_eq!(key(t.file_icon("index.ts")), "dark/typescript.svg");
    assert_eq!(key(t.file_icon("app.test.ts")), "dark/typescript-test.svg");
    assert_eq!(key(t.file_icon("Button.tsx")), "dark/typescript-react.svg");
    assert_eq!(key(t.file_icon("Button.jsx")), "dark/javascript-react.svg");
    // Full file names beat extensions.
    assert_eq!(key(t.file_icon("Cargo.toml")), "dark/cargo.svg");
    assert_eq!(key(t.file_icon("other.toml")), "dark/toml.svg");
    assert_eq!(key(t.file_icon("Cargo.lock")), "dark/cargo-lock.svg");
    assert_eq!(key(t.file_icon("yarn.lock")), "dark/lock.svg");
    assert_eq!(key(t.file_icon("package.json")), "dark/package-json.svg");
    assert_eq!(key(t.file_icon("tsconfig.json")), "dark/typescript-config.svg");
    assert_eq!(key(t.file_icon("data.json")), "dark/json.svg");
    assert_eq!(key(t.file_icon("README.md")), "dark/readme.svg");
    assert_eq!(key(t.file_icon("CLAUDE.md")), "dark/readme.svg");
    assert_eq!(key(t.file_icon("LICENSE")), "dark/license.svg");
    assert_eq!(key(t.file_icon("Dockerfile")), "dark/docker.svg");
    assert_eq!(key(t.file_icon("Makefile")), "dark/makefile.svg");
    // Suffix matching includes compound names like Zed's `eslint.config.js`.
    assert_eq!(key(t.file_icon("eslint.config.js")), "dark/eslint.svg");
    assert_eq!(key(t.file_icon("docker-compose.yml")), "dark/docker.svg");
    // Only the last path component counts.
    assert_eq!(key(t.file_icon("crates/core/Cargo.toml")), "dark/cargo.svg");
    assert_eq!(key(t.file_icon("no_extension")), "dark/file.svg");
    assert_eq!(key(t.file_icon("")), "dark/file.svg");
}

#[test]
fn dotfiles() {
    let t = builtin_dark();
    assert_eq!(key(t.file_icon(".gitignore")), "dark/git.svg");
    assert_eq!(key(t.file_icon("sub/.gitattributes")), "dark/git.svg");
    assert_eq!(key(t.file_icon(".env")), "dark/env.svg");
    assert_eq!(key(t.file_icon(".env.local")), "dark/env.svg");
    assert_eq!(key(t.file_icon(".editorconfig")), "dark/editorconfig.svg");
    assert_eq!(key(t.file_icon(".zshrc")), "dark/bash.svg");
    assert_eq!(key(t.file_icon(".prettierrc")), "dark/prettier.svg");
    // A hidden file with a normal extension falls through to it.
    assert_eq!(key(t.file_icon(".data.json")), "dark/json.svg");
}

#[test]
fn exact_case_first_then_case_insensitive() {
    let t = builtin_dark();
    assert_eq!(key(t.file_icon("readme.md")), "dark/readme.svg");
    assert_eq!(key(t.file_icon("README.MD")), "dark/readme.svg");
    assert_eq!(key(t.file_icon("photo.PNG")), "dark/image.svg");
    assert_eq!(key(t.file_icon("DOCKERFILE")), "dark/docker.svg");
    assert_eq!(key(t.file_icon("api.Dockerfile")), "dark/docker.svg");
    // Longer candidates win even when only their case-folded form matches.
    assert_eq!(key(t.file_icon("Notes.MD")), "dark/markdown.svg");
}

#[test]
fn case_sensitive_pass_wins_over_folded() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let json = tmp.path().join("t.json");
    fs::write(
        &json,
        r#"{ "name": "Case", "author": "", "themes": [ { "name": "Case", "appearance": "dark",
            "file_suffixes": { "R": "r-upper", "r": "r-lower" },
            "file_icons": { "r-upper": { "path": "up.svg" }, "r-lower": { "path": "low.svg" } } } ] }"#,
    )
    .expect("write");
    let t = &load_family(&json).expect("load").themes[0];
    assert_eq!(t.file_icon("a.R"), Some(&file(tmp.path(), "up.svg")));
    assert_eq!(t.file_icon("a.r"), Some(&file(tmp.path(), "low.svg")));
}

#[test]
fn named_directories() {
    let t = builtin_dark();
    assert_eq!(key(t.directory_icon("src", false)), "dark/folder-src.svg");
    assert_eq!(key(t.directory_icon("src", true)), "dark/folder-src-open.svg");
    assert_eq!(key(t.directory_icon("crates/core/tests", false)), "dark/folder-tests.svg");
    assert_eq!(key(t.directory_icon("Docs", false)), "dark/folder-docs.svg");
    assert_eq!(key(t.directory_icon(".github/", false)), "dark/folder-github.svg");
    assert_eq!(key(t.directory_icon("node_modules", true)), "dark/folder-node-open.svg");
    assert_eq!(key(t.directory_icon("target", false)), "dark/folder-dist.svg");
    assert_eq!(key(t.directory_icon(".wormhole", false)), "dark/folder-config.svg");
    assert_eq!(key(t.directory_icon("whatever", false)), "dark/folder.svg");
    assert_eq!(key(t.directory_icon("whatever", true)), "dark/folder-open.svg");
    assert_eq!(key(t.chevron(false)), "dark/chevron-right.svg");
    assert_eq!(key(t.chevron(true)), "dark/chevron-down.svg");
}

#[test]
fn light_and_dark_selection() {
    let family = builtin_family();
    assert_eq!(family.name, "Wormhole Icons");
    let light = pick_theme(family, true).expect("light");
    let dark = pick_theme(family, false).expect("dark");
    assert_eq!(light.appearance, Appearance::Light);
    assert_eq!(dark.appearance, Appearance::Dark);
    assert_eq!(key(light.file_icon("main.rs")), "light/rust.svg");
    // A dark-only family is used for light mode too.
    let (_tmp, ext) = zed_extension();
    let zed = load_family(&ext).expect("load");
    assert_eq!(pick_theme(&zed, true).map(|t| t.appearance), Some(Appearance::Dark));
}

#[test]
fn rejects_paths_escaping_the_theme_folder() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("theme");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(tmp.path().join("secret.svg"), "<svg/>").expect("write");
    let cases = ["../secret.svg", "icons/../../secret.svg", "/etc/passwd"];
    for bad in cases {
        let json = format!(
            r#"{{ "name": "Bad", "author": "", "themes": [ {{ "name": "Bad", "appearance": "dark",
                "file_icons": {{ "default": {{ "path": {bad:?} }} }} }} ] }}"#
        );
        fs::write(dir.join("t.json"), json).expect("write");
        let err = load_family(&dir).expect_err(bad);
        assert!(err.contains("escapes"), "{bad}: {err}");
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(tmp.path().join("secret.svg"), dir.join("link.svg")).expect("symlink");
        fs::write(
            dir.join("t.json"),
            r#"{ "name": "L", "author": "", "themes": [ { "name": "L", "appearance": "dark",
                "file_icons": { "default": { "path": "./link.svg" } } } ] }"#,
        )
        .expect("write");
        assert!(load_family(&dir).is_err());
    }
    // `..` that stays inside is fine.
    fs::write(
        dir.join("t.json"),
        r#"{ "name": "Ok", "author": "", "themes": [ { "name": "Ok", "appearance": "dark",
            "file_icons": { "default": { "path": "icons/../file.svg" } } } ] }"#,
    )
    .expect("write");
    assert_eq!(load_family(&dir).expect("load").themes[0].file_icon("x"), Some(&file(&dir, "file.svg")));
}

#[test]
fn reports_parse_errors_with_position() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let json = tmp.path().join("t.json");
    fs::write(&json, "{ \"name\": \"X\", \"themes\": [ { \"name\": \"X\", \"appearance\": \"dim\" } ] }")
        .expect("write");
    let err = load_family(&json).expect_err("bad appearance");
    assert!(err.contains("line 1"), "{err}");
    assert!(load_family(tmp.path().join("missing").as_path()).is_err());
}

#[test]
fn x_wormhole_render_mode() {
    assert_eq!(builtin_family().render, RenderMode::Color);
    let tmp = tempfile::tempdir().expect("tempdir");
    let json = tmp.path().join("t.json");
    fs::write(
        &json,
        r#"{ "name": "M", "author": "", "x-wormhole": { "render": "mono" },
             "themes": [ { "name": "M", "appearance": "dark" } ] }"#,
    )
    .expect("write");
    let family = load_family(&json).expect("load");
    assert_eq!(family.render, RenderMode::Mono);
    assert_eq!(family.themes[0].render, RenderMode::Mono);

    // A theme made for a version up to 0.2 called the extension `x-andromeda`.
    let text = fs::read_to_string(&json).expect("read").replace("x-wormhole", "x-andromeda");
    fs::write(&json, text).expect("write");
    assert_eq!(load_family(&json).expect("load").render, RenderMode::Mono);
}

#[test]
fn discovers_user_themes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (_ext_tmp, ext) = zed_extension();
    let zed_dst = tmp.path().join("zed-ext");
    fs::create_dir_all(zed_dst.join("icon_themes")).expect("mkdir");
    fs::copy(ext.join("icon_themes/my-icon-theme.json"), zed_dst.join("icon_themes/my-icon-theme.json"))
        .expect("copy");
    let flat = tmp.path().join("flat");
    fs::create_dir_all(&flat).expect("mkdir");
    fs::write(flat.join("a.json"), r#"{ "name": "Alpha", "author": "", "themes": [] }"#).expect("write");
    fs::write(flat.join("notes.txt"), "ignored").expect("write");
    fs::write(tmp.path().join("loose.json"), "{}").expect("write");
    let found = discover(tmp.path());
    let names: Vec<&str> = found.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["Alpha", "My Icon Theme"]);
    assert!(found[1].1.ends_with("zed-ext/icon_themes/my-icon-theme.json"));
    assert!(discover(&tmp.path().join("nope")).is_empty());
}

#[test]
fn bundled_theme_references_only_existing_icons() {
    let family = builtin_family();
    assert_eq!(family.themes.len(), 2, "bundled theme failed to parse");
    for theme in &family.themes {
        assert!(theme.icon_for_type("default").is_some());
        for icon in theme.all_icons() {
            let IconSource::Builtin(k) = icon else { panic!("{icon:?} is not builtin") };
            assert!(builtin_asset(k).is_some(), "{k}");
        }
    }
}

#[test]
fn builtin_table_matches_assets() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icon-themes/wormhole/icons");
    let mut on_disk = Vec::new();
    for variant in ["dark", "light"] {
        for entry in fs::read_dir(base.join(variant)).expect("read_dir") {
            let name = entry.expect("entry").file_name().to_string_lossy().into_owned();
            if name.ends_with(".svg") {
                on_disk.push(format!("{BUILTIN_PREFIX}{variant}/{name}"));
            }
        }
    }
    on_disk.sort();
    let mut table: Vec<String> = builtin_icons().iter().map(|(k, _)| k.to_string()).collect();
    table.sort();
    assert_eq!(table, on_disk, "regenerate src/icon_theme/builtin.rs");
    // Both variants ship the same set of names.
    let dark: Vec<_> = table.iter().filter_map(|k| k.strip_prefix("wormhole-icons/dark/")).collect();
    let light: Vec<_> = table.iter().filter_map(|k| k.strip_prefix("wormhole-icons/light/")).collect();
    assert_eq!(dark, light);
}

#[test]
fn bundled_svgs_are_well_formed() {
    for (k, bytes) in builtin_icons() {
        let text = std::str::from_utf8(bytes).expect("utf-8");
        let text = text.trim();
        assert!(text.starts_with("<svg") && text.ends_with("</svg>"), "{k}");
        let open = &text[..text.find('>').expect("tag end")];
        let vb = open.split("viewBox=\"").nth(1).and_then(|s| s.split('"').next()).expect("viewBox");
        assert!(vb == "0 0 16 16" || vb == "0 0 24 24", "{k}: viewBox {vb}");
        assert!(bytes.len() < 4096, "{k} is {} bytes", bytes.len());
        assert_xml_balanced(k, text);
    }
}

/// Minimal XML well-formedness check (tags balance, no stray `<`), enough
/// for the attribute-only SVGs we ship without pulling in an XML crate.
fn assert_xml_balanced(name: &str, text: &str) {
    let mut stack: Vec<&str> = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        let end = rest[start..].find('>').map(|e| start + e).unwrap_or_else(|| panic!("{name}: open tag"));
        let tag = &rest[start + 1..end];
        assert!(!tag.contains('<'), "{name}: stray <");
        if tag.starts_with('?') || tag.starts_with('!') {
        } else if let Some(close) = tag.strip_prefix('/') {
            assert_eq!(stack.pop(), Some(close.trim()), "{name}: mismatched </{close}>");
        } else if !tag.ends_with('/') {
            let tag_name = tag.split_whitespace().next().unwrap_or_default();
            stack.push(tag_name);
        }
        let quotes = tag.matches('"').count();
        assert!(quotes.is_multiple_of(2), "{name}: unbalanced quotes in <{tag}>");
        rest = &rest[end + 1..];
    }
    assert!(stack.is_empty(), "{name}: unclosed {stack:?}");
}

fn parses() -> usize {
    PARSES.with(|n| n.get())
}

#[test]
fn discovery_and_families_are_read_once_until_they_change() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let folder = tmp.path().join("mini");
    fs::create_dir_all(&folder).expect("mkdir");
    let json = folder.join("t.json");
    let theme = |name: &str| {
        format!(
            r#"{{ "name": "{name}", "author": "", "themes": [ {{ "name": "{name}", "appearance": "dark" }} ] }}"#
        )
    };
    fs::write(&json, theme("Mini")).expect("write");

    let start = parses();
    assert_eq!(discover(tmp.path()), [("Mini".to_string(), json.clone())]);
    assert_eq!(discover(tmp.path()), [("Mini".to_string(), json.clone())]);
    assert_eq!(parses() - start, 1);
    let first = load_family(&folder).expect("load");
    let again = load_family(&json).expect("load");
    assert_eq!(parses() - start, 2);
    // The same theme, shared rather than copied.
    assert!(Arc::ptr_eq(&first.themes[0], &again.themes[0]));

    fs::write(&json, theme("Mini Two")).expect("write");
    assert_eq!(discover(tmp.path())[0].0, "Mini Two");
    let changed = load_family(&json).expect("load");
    assert_eq!(changed.name, "Mini Two");
    assert_eq!(parses() - start, 4);
}

#[test]
fn themes_are_found_by_name_and_fall_back_to_the_bundled_one() {
    let (_tmp, ext) = zed_extension();
    let user_dir = ext.parent().expect("user dir");
    let zed = theme_named(user_dir, "My Icon Theme", true).expect("theme");
    assert_eq!(zed.name, "My Icon Theme");
    let bundled = pick_shared(builtin_family(), true).expect("bundled");
    for name in ["Wormhole Icons", "Missing"] {
        let theme = theme_named(user_dir, name, true).expect("theme");
        assert!(Arc::ptr_eq(&theme, &bundled), "{name}");
    }
    assert_eq!(theme_named(user_dir, "Wormhole Icons", false).map(|t| t.appearance), Some(Appearance::Dark));
}

/// `cargo test -p wormhole-core --lib icon_theme_timing -- --ignored --nocapture`
#[test]
#[ignore = "timing, run by hand"]
fn icon_theme_timing() {
    let (_tmp, ext) = zed_extension();
    let user_dir = ext.parent().expect("user dir");
    let runs = 1000;
    let time = |f: &dyn Fn()| {
        let start = std::time::Instant::now();
        for _ in 0..runs {
            f();
        }
        start.elapsed() / runs
    };
    let copy = time(&|| drop(std::hint::black_box(pick_theme(builtin_family(), true).cloned())));
    let shared = time(&|| drop(std::hint::black_box(theme_named(user_dir, "Wormhole Icons", true))));
    let user = time(&|| drop(std::hint::black_box(theme_named(user_dir, "My Icon Theme", true))));
    eprintln!("bundled theme: copy {copy:?}, shared {shared:?}; user theme by name (cached) {user:?}");
}
