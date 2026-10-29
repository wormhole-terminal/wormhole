//! Tests of the format as a whole: versions, strict and lenient reading, the copy step, the
//! conversion of a version 0 theme, and the saved fixture themes. The parts have tests of their
//! own beside them.

use super::copy::{convert_to_v1, copy_base, copy_builtin_base, diff_base};
use super::{CURRENT_VERSION, KEY_TABLE, read_theme_file, slug};
use crate::theme::tokens::test_support::{no_lookup, resolve_str, snapshot, tokyo_palette_block, v1_text};
use crate::theme::tokens::{MAX_THEME_BYTES, ThemeFile, builtin_tokens, resolve, variants};

const V0_EXTENDS: &str = include_str!("../../../../tests/fixtures/themes/v0/extends_nord.json");
const V0_STANDALONE: &str = include_str!("../../../../tests/fixtures/themes/v0/standalone.json");
const V1_BASED: &str = include_str!("../../../../tests/fixtures/themes/v1/based.json");
const V1_PAIR: &str = include_str!("../../../../tests/fixtures/themes/v1/pair.json");

fn read(text: &str) -> ThemeFile {
    read_theme_file(text).unwrap_or_else(|e| panic!("{e}"))
}

fn hex(t: &crate::theme::tokens::ThemeTokens, name: &str) -> String {
    t.get(name).unwrap_or_else(|| panic!("{name}")).to_hex()
}

// The fixtures. What they SET must resolve to these values in every later release; colors the
// rules work out are not asserted here (a rule may be improved), only values a file wrote.

#[test]
fn fixture_v0_extends_keeps_resolving_to_what_it_sets() {
    let f = read(V0_EXTENDS);
    assert_eq!((f.read.source_version, f.version), (0, None));
    let t = resolve(&f, &no_lookup).unwrap();
    assert_eq!(t.name, "Fixture Night");
    assert_eq!(hex(&t, "terminal.background"), "#101820");
    assert_eq!(hex(&t, "terminal.selection_background"), "#264060");
    assert_eq!(hex(&t, "terminal.ansi.4"), "#5599ff");
    assert_eq!(hex(&t, "terminal.ansi.9"), "#ff6677");
    assert_eq!(hex(&t, "primary"), "#e07020");
    // `accent.hover` is migrated to `primary.hover`.
    assert_eq!(hex(&t, "primary.hover"), "#f09040");
    assert!(t.syntax["keyword"].italic && t.syntax["keyword"].color.to_hex() == "#cc88ff");
    assert_eq!(hex(&t, "syntax.string"), "#88cc66");
    assert_eq!((t.metrics.radius, t.metrics.radius_sm, t.metrics.focus_ring_width), (9.0, 3.0, 3.0));
    assert_eq!(t.metrics.ui_font_family.as_deref(), Some("Inter"));
    // Not set by the file: Nord's.
    assert_eq!(
        hex(&t, "terminal.foreground"),
        hex(&builtin_tokens().into_iter().find(|t| t.name == "Nord").unwrap(), "terminal.foreground")
    );
    assert!(f.read.notices.is_empty());
}

#[test]
fn fixture_v0_standalone_keeps_resolving_to_what_it_sets() {
    let t = resolve(&read(V0_STANDALONE), &no_lookup).unwrap();
    assert_eq!(hex(&t, "terminal.background"), "#fafafa");
    assert_eq!(hex(&t, "terminal.ansi.15"), "#aaaaaa");
    assert_eq!(hex(&t, "primary"), "#0066cc");
    assert_eq!((t.metrics.radius, t.metrics.border_width), (4.0, 2.0));
    assert_eq!(t.appearance, crate::theme::tokens::Appearance::Light);
}

#[test]
fn fixture_v1_based_keeps_resolving_to_what_it_sets() {
    let f = read(V1_BASED);
    assert_eq!((f.read.source_version, f.version, f.id.as_deref()), (1, Some(1), Some("fixture_based")));
    let t = resolve(&f, &no_lookup).unwrap();
    // The person's own values win over the base's.
    assert_eq!(hex(&t, "terminal.cursor_text"), "#000000");
    assert_eq!(hex(&t, "terminal.selection_background"), "#335577");
    // The base supplies the rest, and its expression is evaluated here: primary is ansi.4.
    assert_eq!(hex(&t, "terminal.background"), "#2e3440");
    assert_eq!(hex(&t, "primary"), "#81a1c1");
    assert_eq!(hex(&t, "git.branch"), "#aa66ff");
    assert_eq!(t.appearance, crate::theme::tokens::Appearance::Dark);
    assert_eq!(t.syntax["keyword"].color.to_hex(), "#cc88ff");
    assert_eq!(
        (t.metrics.radius, t.metrics.radius_sm, t.metrics.focus_ring_width, t.metrics.ui_font_size),
        (8.0, 2.0, 3.0, 14.0)
    );
    // Metrics the file does not set are Wormhole's.
    assert_eq!(t.metrics.border_width, 1.0);
}

#[test]
fn fixture_v1_pair_has_two_versions_each_with_its_own_base() {
    let f = read(V1_PAIR);
    let all = variants(&f, "stem").unwrap();
    let names: Vec<_> = all.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, ["Fixture Pair Light", "Fixture Pair Dark"]);
    let slugs: Vec<_> = all.iter().map(|v| v.slug.as_deref()).collect();
    assert_eq!(slugs, [Some("fixture_pair.light"), Some("fixture_pair.dark")]);
    let (light, dark) =
        (resolve(&all[0].file, &no_lookup).unwrap(), resolve(&all[1].file, &no_lookup).unwrap());
    assert_eq!(
        (hex(&light, "terminal.background"), hex(&dark, "terminal.background")),
        ("#ffffff".into(), "#0d1117".into())
    );
    // The shared primary reaches both, over each base; each has its own branch color.
    assert_eq!((hex(&light, "primary"), hex(&dark, "primary")), ("#c2410c".into(), "#c2410c".into()));
    assert_eq!((hex(&light, "git.branch"), hex(&dark, "git.branch")), ("#7a3cc4".into(), "#b78cff".into()));
    assert_eq!((light.metrics.radius, dark.metrics.radius), (8.0, 8.0));
    // `resolve` of the whole file shows its first version.
    assert_eq!(resolve(&f, &no_lookup).unwrap().name, "Fixture Pair Light");
}

#[test]
fn version_1_files_are_written_and_read_back_the_same() {
    for text in [V1_BASED, V1_PAIR] {
        let f = read(text);
        let written = f.to_json_pretty();
        assert!(
            written.contains("\"cursor.text\"") || written.contains("\"selection.background\""),
            "{written}"
        );
        assert!(!written.contains("cursor_text") && !written.contains("\"ansi\":"), "{written}");
        let back = read(&written);
        assert_eq!(back.to_json_pretty(), written, "stable");
        let name = f.name.clone();
        assert_eq!(back.name, name);
        assert_eq!(
            snapshot(&resolve(&back, &no_lookup).unwrap()),
            snapshot(&resolve(&f, &no_lookup).unwrap())
        );
    }
}

// Versions.

#[test]
fn a_missing_version_is_0_and_a_bad_one_is_an_error() {
    assert_eq!(read(r#"{"name": "A"}"#).read.source_version, 0);
    assert_eq!(read(r#"{"version": 0, "name": "A"}"#).read.source_version, 0);
    for bad in [r#""1""#, "-1", "1.5", "null", "true", "[1]", "1e2"] {
        let e = read_theme_file(&format!(r#"{{"version": {bad}}}"#)).unwrap_err();
        assert!(e.contains("version: expected a whole number"), "{bad}: {e}");
    }
}

#[test]
fn version_0_and_1_are_strict() {
    let e = read_theme_file(r#"{"nmae": "A"}"#).unwrap_err();
    assert!(e.contains("unknown field `nmae`"), "{e}");
    let e = read_theme_file(r#"{"version": 1, "nmae": "A"}"#).unwrap_err();
    assert!(e.contains("unknown key \"nmae\" (did you mean `name`?)"), "{e}");
    let e = read_theme_file(r##"{"version": 1, "terminal": {"cursr": "#fff"}}"##).unwrap_err();
    assert!(e.contains("cursr"), "{e}");
    let e = read_theme_file(r#"{"version": 1, "syntax": {"keyword": {"itallic": true}}}"#).unwrap_err();
    assert!(e.contains("itallic") && e.contains("italic"), "{e}");
    let e = read_theme_file(r#"{"version": 1, "metrics": {"radius": 500}}"#).unwrap_err();
    assert!(e.contains("out of range"), "{e}");
    let e = read_theme_file(r#"{"version": 1, "ui": {"primary": 5}}"#).unwrap_err();
    assert!(e.contains("expected a string"), "{e}");
    let e = read_theme_file(r#"{"version": 1, "extends": "Nord"}"#).unwrap_err();
    assert!(e.contains("`base`"), "{e}");
    let e = read_theme_file(r#"{"version": 1, "id": "My Theme"}"#).unwrap_err();
    assert!(e.contains("not a slug"), "{e}");
    let e = read_theme_file(r#"{"version": 1, "variants": [{"name": "Light"}]}"#).unwrap_err();
    assert!(e.contains("lowercase"), "{e}");
    let e = read_theme_file(r#"{"version": 1, "base": {"terminal": {}}}"#).unwrap_err();
    assert!(e.contains("`from`"), "{e}");
}

#[test]
fn an_old_key_in_a_version_1_file_says_its_new_name() {
    for r in KEY_TABLE {
        let text = match r.block {
            "terminal" => format!(r##"{{"version": 1, "terminal": {{"{}": "#fff"}}}}"##, r.old),
            _ => format!(r#"{{"version": 1, "metrics": {{"{}": 3}}}}"#, r.old),
        };
        let e = read_theme_file(&text).unwrap_err();
        assert!(e.contains(&format!("`{}`", r.new)), "{}: {e}", r.old);
    }
    let e = read_theme_file(r##"{"version": 1, "ui": {"border": "terminal.cursor_text"}}"##).unwrap_err();
    assert!(e.contains("terminal.cursor.text"), "{e}");
    assert!(read_theme_file(r##"{"version": 1, "terminal": {"ansi": {"4": "#fff"}}}"##).is_err());
}

#[test]
fn every_renamed_key_reads_into_the_field_it_names() {
    let f = read(
        r##"{"version": 1,
        "terminal": {"cursor.text": "#010101", "selection.background": "#020202", "selection.foreground": "#030303", "ansi.5": "#040404"},
        "metrics": {"radius": 1, "radius.sm": 2, "radius.chip": 3, "radius.lg": 4, "spacing": 5, "border.width": 6,
                    "divider.width": 7, "focus_ring.width": 8, "ui_font.family": "F", "ui_font.size": 9}}"##,
    );
    let (t, m) = (&f.terminal, &f.metrics);
    assert_eq!(
        (
            t.cursor_text.as_deref(),
            t.selection_background.as_deref(),
            t.selection_foreground.as_deref(),
            t.ansi["5"].as_str()
        ),
        (Some("#010101"), Some("#020202"), Some("#030303"), "#040404")
    );
    assert_eq!(
        [
            m.radius,
            m.radius_sm,
            m.radius_chip,
            m.radius_lg,
            m.spacing,
            m.border_width,
            m.divider_width,
            m.focus_ring_width,
            m.ui_font_size
        ],
        [1, 2, 3, 4, 5, 6, 7, 8, 9].map(|n| Some(n as f32))
    );
    assert_eq!(m.ui_font_family.as_deref(), Some("F"));
}

#[test]
fn a_newer_file_loads_what_it_knows_and_counts_the_rest() {
    let f = read(
        r##"{"version": 7, "name": "Aurora", "glow": true,
        "terminal": {"background": "#101010", "cursor.shape": "block", "ansi.99": "#fff"},
        "ui": {"primary": "#ff8800", "tint.ultra": "#fff", "border": "wobble(primary)"},
        "syntax": {"keyword": {"color": "#fff", "blink": true}},
        "metrics": {"radius": 5, "radius.huge": 9, "spacing": 9000}}"##,
    );
    assert_eq!((f.read.source_version, f.read.newer, f.version), (7, true, Some(CURRENT_VERSION)));
    // glow, cursor.shape, ansi.99, tint.ultra, border (bad expression), blink, radius.huge, spacing.
    assert_eq!(f.read.skipped, 8);
    assert_eq!(f.terminal.background.as_deref(), Some("#101010"));
    assert_eq!(f.ui.len(), 1);
    assert_eq!(f.metrics.radius, Some(5.0));
    assert_eq!(f.metrics.spacing, None, "out of range stays out of range");
    // The same file as version 1 is an error, not a count.
    assert!(read_theme_file(r#"{"version": 1, "glow": true}"#).is_err());
}

#[test]
fn a_newer_file_drops_an_expression_that_names_a_color_this_version_lacks() {
    let text = v1_text(r##""ui": {"primary": "#ff8800", "border": "mix(primary, glow.edge, 30%)"}"##)
        .replace("\"version\": 1", "\"version\": 5");
    let f = read(&text);
    assert_eq!(f.read.skipped, 1);
    assert_eq!(f.ui.len(), 1);
    assert!(resolve_str(&text).is_ok(), "the rest of the theme still resolves");
    let strict = text.replace("\"version\": 5", "\"version\": 1");
    assert!(resolve_str(&strict).is_err(), "version 1 stays strict");
}

#[test]
fn a_newer_file_keeps_the_caps() {
    let big = format!(r#"{{"version": 9, "name": "{}"}}"#, "x".repeat(MAX_THEME_BYTES));
    assert!(read_theme_file(&big).unwrap_err().contains("larger than"));
    let many = (0..40).map(|i| format!(r#"{{"name": "v{i}"}}"#)).collect::<Vec<_>>().join(",");
    let f = read(&format!(r#"{{"version": 9, "variants": [{many}]}}"#));
    assert_eq!(f.variants.len(), crate::theme::tokens::MAX_VARIANTS);
    assert_eq!(f.read.skipped, 8);
    let e = read_theme_file(&format!(r#"{{"version": 1, "variants": [{many}]}}"#)).unwrap_err();
    assert!(e.contains("more than"), "{e}");
}

#[test]
fn a_newer_file_still_needs_opaque_terminal_colors() {
    let newer = v1_text("").replace("\"version\": 1", "\"version\": 4");
    assert_eq!(resolve_str(&newer).unwrap().name, "T");
    let translucent = newer.replacen("\"#1a1b26\"", "\"#1a1b2680\"", 1);
    assert!(resolve_str(&translucent).unwrap_err().contains("must be opaque"));
}

#[test]
fn a_key_given_twice_is_an_error_in_version_1_only() {
    let e = read_theme_file(r##"{"version": 1, "ui": {"primary": "#f00", "primary": "#0f0"}}"##).unwrap_err();
    assert!(e.contains("\"primary\" is given twice"), "{e}");
    let e = read_theme_file(r##"{"version": 3, "name": "a", "name": "b"}"##).unwrap_err();
    assert!(e.contains("given twice"), "{e}");
    // As ever for version 0: the last wins.
    assert_eq!(read(r##"{"ui": {"primary": "#f00", "primary": "#0f0"}}"##).ui["primary"], "#0f0");
}

// Roots.

#[test]
fn a_version_1_theme_never_falls_back_to_tokyo_night() {
    let e = resolve_str(r##"{"version": 1, "name": "Bare", "ui": {"primary": "#f00"}}"##).unwrap_err();
    assert!(
        e.contains("terminal colors not set") && e.contains("ansi.3") && e.contains("selection.background"),
        "{e}"
    );
    // One missing color is reported, not filled.
    let palette = format!(
        "{{{}}}",
        tokyo_palette_block()
            .trim_matches(|c| c == '{' || c == '}')
            .split(", ")
            .filter(|p| !p.starts_with("\"ansi.9\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let e = resolve_str(&format!(r#"{{"version": 1, "terminal": {palette}}}"#)).unwrap_err();
    assert!(e.contains("ansi.9"), "{e}");
    // With the palette complete, the optional two follow the background and foreground.
    let palette = tokyo_palette_block();
    let with =
        |extra: &str| resolve_str(&format!(r##"{{"version": 1, "terminal": {palette}{extra}}}"##)).unwrap();
    let t = with("");
    let t2 = resolve_str(&v1_text("")).unwrap();
    assert_eq!(snapshot(&t), snapshot(&t2));
}

#[test]
fn cursor_text_and_selection_foreground_follow_when_unset() {
    let palette = tokyo_palette_block();
    // Drop the two optional ones from the block.
    let only: String = palette
        .trim_matches(|c| c == '{' || c == '}')
        .split(", ")
        .filter(|p| {
            !p.contains("cursor.text")
                && !p.contains("selection.foreground")
                && !p.starts_with("\"background\"")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let t = resolve_str(&format!(r##"{{"version": 1, "terminal": {{{only}, "background": "#102030"}}}}"##))
        .unwrap();
    assert_eq!(hex(&t, "terminal.cursor_text"), "#102030");
    assert_eq!(hex(&t, "terminal.selection_foreground"), hex(&t, "terminal.foreground"));
}

// The copy step.

fn nord() -> crate::theme::tokens::ThemeTokens {
    builtin_tokens().into_iter().find(|t| t.name == "Nord").unwrap()
}

#[test]
fn a_copied_palette_is_complete_and_freezes_nothing_else() {
    let base = copy_builtin_base("nord", &no_lookup, "2026-10-29").unwrap();
    assert_eq!((base.from.as_str(), base.copied.as_deref()), ("nord", Some("2026-10-29")));
    let t = &base.file.terminal;
    assert_eq!(t.ansi.len(), 16);
    assert!(t.foreground.is_some() && t.background.is_some() && t.cursor.is_some());
    assert!(t.cursor_text.is_some() && t.selection_background.is_some() && t.selection_foreground.is_some());
    assert!(base.file.metrics.is_empty(), "defaults are not frozen");
    assert_eq!(base.file.appearance, None, "detected, not frozen");
    assert!(base.file.ui.is_empty());
}

#[test]
fn a_theme_with_nothing_of_its_own_on_a_copied_base_is_the_source() {
    for name in ["nord", "wormhole.light", "wormhole.dark", "github_light", "soltari.light", "tokyo_night"] {
        let base = copy_builtin_base(name, &no_lookup, "2026-10-29").unwrap();
        let file = ThemeFile {
            version: Some(1),
            name: Some("Copy".into()),
            base: Some(Box::new(base)),
            ..ThemeFile::default()
        };
        let copy = resolve(&file, &no_lookup).unwrap();
        let source = crate::theme::registry::builtin_named(name).unwrap();
        assert_eq!(snapshot(&copy), snapshot(source), "{name}");
    }
}

#[test]
fn a_copy_keeps_expressions_as_written_and_the_appearance_the_source_sets() {
    let light = copy_builtin_base("wormhole.light", &no_lookup, "2026-10-29").unwrap();
    assert!(
        light.file.ui.values().any(|v| v.contains('(') || v.contains('/')),
        "expressions stay expressions: {:?}",
        light.file.ui
    );
    assert!(light.file.ui.contains_key("primary"));
    assert!(light.file.appearance.is_some());
    // Written out, the base survives a round trip.
    let file = ThemeFile { version: Some(1), base: Some(Box::new(light.clone())), ..ThemeFile::default() };
    assert_eq!(read(&file.to_json_pretty()).base.as_deref(), Some(&light));
    assert!(copy_builtin_base("nope", &no_lookup, "d").is_err());
}

#[test]
fn a_theme_without_extends_is_copied_with_the_palette_it_resolves_to() {
    // Tokyo Night sits under it implicitly; the copy writes that palette out.
    let f = read(r##"{"name": "Plain", "terminal": {"background": "#000000"}, "ui": {"primary": "#f0f"}}"##);
    let base = copy_base("plain", &f, &no_lookup, "2026-10-29").unwrap();
    assert_eq!(base.file.terminal.background.as_deref(), Some("#000000"), "as written");
    assert_eq!(base.file.terminal.ansi.len(), 16);
    assert!(base.file.terminal.cursor_text.is_some());
    assert_eq!(base.file.ui["primary"], "#f0f");
}

#[test]
fn a_copy_leaves_out_cursor_text_and_selection_foreground_the_source_does_not_set() {
    let palette = tokyo_palette_block();
    let only: String = palette
        .trim_matches(|c| c == '{' || c == '}')
        .split(", ")
        .filter(|p| !p.contains("cursor.text") && !p.contains("selection.foreground"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = read(&format!(r##"{{"version": 1, "name": "S", "terminal": {{{only}}}}}"##));
    let base = copy_base("s", &source, &no_lookup, "2026-10-29").unwrap();
    let t = &base.file.terminal;
    assert_eq!((&t.cursor_text, &t.selection_foreground), (&None, &None), "they keep following");
    assert_eq!(t.ansi.len(), 16, "the rest of the palette is complete");
    // Set by the source, they are copied.
    let nord = copy_builtin_base("nord", &no_lookup, "d").unwrap();
    assert!(nord.file.terminal.cursor_text.is_some() && nord.file.terminal.selection_foreground.is_some());
}

#[test]
fn a_chain_is_copied_merged_with_the_nearest_on_top() {
    let parent = read(
        r##"{"name": "P", "extends": "Nord", "ui": {"primary": "#111111", "border": "#222222"}, "metrics": {"radius": 3}}"##,
    );
    let child = read(r##"{"name": "C", "extends": "P", "ui": {"primary": "#333333"}}"##);
    let lookup = |n: &str| (n == "P").then(|| parent.clone());
    let base = copy_base("c", &child, &lookup, "d").unwrap();
    assert_eq!(base.file.ui["primary"], "#333333");
    assert_eq!(base.file.ui["border"], "#222222");
    assert_eq!(base.file.metrics.radius, Some(3.0));
}

fn same(a: &ThemeFile, b: &ThemeFile) {
    let (x, y) = (resolve(a, &no_lookup).unwrap(), resolve(b, &no_lookup).unwrap());
    assert_eq!(snapshot(&x), snapshot(&y));
}

#[test]
fn converting_a_version_0_theme_changes_nothing_that_is_shown() {
    let slug_of = |n: &str| slug::builtin_slug(n).unwrap_or("x").to_string();
    for text in [V0_EXTENDS, V0_STANDALONE, r##"{"name": "My Theme"}"##] {
        let v0 = read(text);
        let v1 = convert_to_v1(&v0, &no_lookup, &slug_of, "my_id", "2026-10-29").unwrap();
        assert_eq!((v1.version, v1.id.as_deref()), (Some(1), Some("my_id")));
        assert_eq!(v1.name, v0.name);
        assert!(v1.extends.is_none() && v1.base.is_some());
        same(&v0, &v1);
        // And as text.
        same(&v0, &read(&v1.to_json_pretty()));
        // The person's own values stay on top, as they were.
        assert_eq!(v1.ui, v0.ui);
    }
}

#[test]
fn converting_a_user_theme_that_extends_another_user_theme() {
    let parent = read(r##"{"name": "Mine", "extends": "Dracula", "ui": {"primary": "#aa5500"}}"##);
    let child = read(r##"{"name": "Kid", "extends": "Mine", "ui": {"git.branch": "#00aa55"}}"##);
    let lookup = |n: &str| (n == "Mine").then(|| parent.clone());
    let slug_of = |n: &str| if n == "Mine" { "mine".into() } else { "x".to_string() };
    let v1 = convert_to_v1(&child, &lookup, &slug_of, "kid", "d").unwrap();
    assert_eq!(v1.base.as_ref().unwrap().from, "mine");
    assert_eq!(snapshot(&resolve(&child, &lookup).unwrap()), snapshot(&resolve(&v1, &no_lookup).unwrap()));
}

#[test]
fn converting_a_family_keeps_every_version() {
    let family = read(
        r##"{"name": "Duo", "extends": "Nord", "ui": {"primary": "#c08a1e"}, "syntax": {"keyword": {"italic": true}},
        "variants": [
          {"name": "Dark", "appearance": "dark", "terminal": {"background": "#15131c"}},
          {"name": "Light", "appearance": "light", "terminal": {"background": "#f6f2ea", "foreground": "#2b2a35"}},
          {"name": "Light Minimal", "extends": "Light", "metrics": {"radius": 2}},
          {"name": "Tokyo", "extends": "Tokyo Night"}]}"##,
    );
    let v1 =
        convert_to_v1(&family, &no_lookup, &|n| slug::builtin_slug(n).unwrap_or("x").to_string(), "duo", "d")
            .unwrap();
    let names: Vec<_> = v1.variants.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, ["dark", "light", "light_minimal", "tokyo"]);
    let (old, new) = (variants(&family, "s").unwrap(), variants(&v1, "s").unwrap());
    for (o, n) in old.iter().zip(&new) {
        assert_eq!(
            snapshot(&resolve(&o.file, &no_lookup).unwrap()),
            snapshot(&resolve(&n.file, &no_lookup).unwrap()),
            "{}",
            o.name
        );
    }
    // And through the text of the file.
    let back = variants(&read(&v1.to_json_pretty()), "s").unwrap();
    assert_eq!(back.len(), 4);
}

#[test]
fn the_base_is_compared_with_its_source_now() {
    let now = copy_builtin_base("nord", &no_lookup, "2026-11-01").unwrap();
    let mut stored = now.clone();
    assert!(diff_base(&stored, &now, &ThemeFile::default()).is_empty());
    // The source changed since: a terminal color, a ui value, a metric, and a value that is gone.
    stored.file.terminal.ansi.insert("3".into(), "#123456".into());
    stored.file.ui.insert("primary".into(), "#abcdef".into());
    stored.file.metrics.radius = Some(7.0);
    stored.file.terminal.cursor_text = None;
    let mut own = ThemeFile::default();
    own.ui.insert("primary".into(), "#ff0000".into());
    let changes = diff_base(&stored, &now, &own);
    let keys: Vec<_> = changes.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, ["metrics.radius", "terminal.ansi.3", "terminal.cursor.text", "ui.primary"]);
    let get = |k: &str| changes.iter().find(|c| c.key == k).unwrap();
    assert_eq!(
        (get("terminal.ansi.3").old.as_deref(), get("terminal.ansi.3").kept),
        (Some("#123456"), false)
    );
    assert!(get("terminal.ansi.3").new.is_some());
    assert_eq!(get("metrics.radius").new, None);
    assert_eq!(get("terminal.cursor.text").old, None);
    assert!(get("ui.primary").kept, "the person's own value hides it");
    assert_eq!(get("ui.primary").new, None);
}

#[test]
fn the_nord_fixture_is_what_the_table_of_slugs_says() {
    assert_eq!(nord().name, slug::builtin_name("nord").unwrap());
}

// Slugs in the settings and the registry.

#[test]
fn settings_accept_slugs_beside_names_and_paths() {
    use crate::theme::registry::{ThemeId, UserThemes, builtin_named};
    let dir = tempfile::tempdir().unwrap();
    let put = |file: &str, text: &str| std::fs::write(dir.path().join(file), text).unwrap();
    put("a.json", V1_BASED);
    put("b.json", V1_PAIR);
    put("c.json", r#"{"name": "My Theme", "extends": "Nord"}"#);
    put("d.json", r#"{"name": "My-Theme", "extends": "Nord"}"#);
    put("e.json", r#"{"name": "Nord", "extends": "Nord"}"#);
    let themes = UserThemes::read(dir.path());
    let slugs: Vec<_> = themes.themes.iter().map(|t| (t.name.as_str(), t.slug.as_str())).collect();
    assert_eq!(
        slugs,
        [
            ("Fixture Based", "fixture_based"),
            ("Fixture Pair Light", "fixture_pair.light"),
            ("Fixture Pair Dark", "fixture_pair.dark"),
            ("My Theme", "my_theme"),
            ("My-Theme", "my_theme_2"),
            ("Nord", "nord_2"),
        ]
    );
    // Clashes are reported, not hidden.
    assert!(
        themes.themes[3].id_clash.is_none()
            && themes.themes[4].id_clash.is_some()
            && themes.themes[5].id_clash.is_some()
    );
    // A slug selects the theme, a name still does.
    assert!(matches!(themes.select("fixture_pair.dark"), Some(ThemeId::Variant(_, s)) if s == "dark"));
    assert_eq!(themes.select("my_theme_2"), themes.select("My-Theme"));
    assert!(matches!(themes.select("fixture_based"), Some(ThemeId::File(_))));
    // The built-ins by slug, as `settings.theme` may now hold them.
    assert_eq!(builtin_named("catppuccin_latte").unwrap().name, "Catppuccin Latte");
    assert_eq!(builtin_named("wormhole.light").unwrap().name, "Wormhole Light");
    assert_eq!(builtin_named("Nord").unwrap().name, "Nord");
    // Hiding by either spelling is hiding the same theme.
    let mut s = crate::config::Settings::default();
    assert!(s.set_theme_hidden("nord", true) && s.theme_hidden("Nord") && !s.set_theme_hidden("Nord", true));
    assert!(s.set_theme_hidden("Nord", false) && s.hidden_themes.is_empty());
}
