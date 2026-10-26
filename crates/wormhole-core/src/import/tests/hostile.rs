//! What a hostile dotfiles repository can cost: deep nesting, patterns that backtrack, numbers that
//! are not numbers, palettes that are not whole, files the settings name outside the home folder.
//! Each is a file a person might have cloned and linked into their home, read for them by Import.

use std::path::PathBuf;

use super::{Fixture, number, palette, skipped};
use crate::import::{Reason, Setting, Source};

/// A file outside the home folder (but in the fixture's own temporary folder).
fn outside(fixture: &Fixture, name: &str, text: &str) -> PathBuf {
    fixture.bytes(&fixture.root().join("outside").join(name), text.as_bytes())
}

fn left_alone(fixture: &Fixture, source: Source) -> crate::import::Found {
    let found = fixture.found(source);
    assert!(
        found.proposal.skipped.iter().any(|skip| skip.reason == Reason::OutsideHome),
        "{:?}",
        found.proposal.skipped
    );
    found
}

// ---------------------------------------------------------------- nesting

#[test]
fn a_lua_table_nested_without_end_is_cut_off_not_followed() {
    let fixture = Fixture::new();
    let deep = 20_000;
    fixture.home(
        ".config/wezterm/wezterm.lua",
        &format!(
            "config.font_size = 14\nconfig.colors = {}{}\nconfig.font_rules = {}{}\n",
            "{".repeat(deep),
            "}".repeat(deep),
            "{ a = ".repeat(deep),
            "1 }".repeat(deep)
        ),
    );
    let p = fixture.found(Source::WezTerm).proposal;
    // Reading went on after the nest, and nothing overflowed.
    assert_eq!(number(&p, Setting::FontSize), 14.0);
}

#[test]
fn lua_calls_nested_without_end_are_dynamic() {
    let fixture = Fixture::new();
    let deep = 20_000;
    fixture.home(
        ".config/wezterm/wezterm.lua",
        &format!(
            "config.font = {}'JetBrains Mono'{}\nconfig.font_size = 13\n",
            "wezterm.font(".repeat(deep),
            ")".repeat(deep)
        ),
    );
    let p = fixture.found(Source::WezTerm).proposal;
    assert!(skipped(&p, "font", &Reason::DynamicLua));
    assert_eq!(number(&p, Setting::FontSize), 13.0);
}

#[test]
fn an_ordinary_nest_of_tables_is_still_read() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/wezterm/wezterm.lua",
        "config.colors = { foreground = '#ffffff', background = '#000000', ansi = { '#111111' } }\n",
    );
    let p = fixture.found(Source::WezTerm).proposal;
    assert_eq!(palette(&p, Setting::Theme).palette[0], 0x111111);
}

#[test]
fn a_warp_theme_nested_deep_costs_a_line_each() {
    let fixture = Fixture::new();
    let mut theme = String::from("background: '#000000'\nforeground: '#ffffff'\nterminal_colors:\n");
    for depth in 1..1000 {
        theme.push_str(&format!("{}k{depth}:\n", " ".repeat(depth)));
    }
    theme.push_str(&format!("a: {}\n", "{ b: ".repeat(5000) + "1" + &" }".repeat(5000)));
    fixture.home(".warp/themes/deep.yaml", &theme);
    fixture.home(
        ".warp/settings.toml",
        "[appearance.themes]\ntheme = { custom = { name = \"Deep\", path = \"deep.yaml\" } }\n",
    );
    let p = fixture.found(Source::Warp).proposal;
    assert_eq!(palette(&p, Setting::Theme).background, 0x000000);
}

// ---------------------------------------------------------------- patterns

#[test]
fn a_glob_with_many_stars_takes_linear_time() {
    let fixture = Fixture::new();
    let name = format!("{}.conf", "a".repeat(48));
    fixture.home(&format!(".config/kitty/themes/{name}"), "font_size 19\n");
    // Thirty `*a` and a letter the name does not have: exponential for a recursive matcher.
    let pattern = format!("{}z.conf", "*a".repeat(30));
    fixture.home(".config/kitty/kitty.conf", &format!("font_size 12\nglobinclude themes/{pattern}\n"));
    let p = fixture.found(Source::Kitty).proposal;
    assert_eq!(number(&p, Setting::FontSize), 12.0, "no match, and an answer");

    // The same stars with a letter it has do match.
    fixture.home(
        ".config/kitty/kitty.conf",
        &format!("font_size 12\nglobinclude themes/{}.conf\n", "*a".repeat(30)),
    );
    assert_eq!(number(&fixture.found(Source::Kitty).proposal, Setting::FontSize), 19.0);
}

#[test]
fn a_glob_pattern_is_capped_in_length() {
    let fixture = Fixture::new();
    fixture.home(".config/kitty/themes/night.conf", "font_size 19\n");
    fixture.home(
        ".config/kitty/kitty.conf",
        &format!("font_size 12\nglobinclude themes/{}.conf\n", "*".repeat(300)),
    );
    assert_eq!(number(&fixture.found(Source::Kitty).proposal, Setting::FontSize), 12.0);
    // The same file, a pattern of a sensible length.
    fixture.home(".config/kitty/kitty.conf", "font_size 12\nglobinclude themes/*.conf\n");
    assert_eq!(number(&fixture.found(Source::Kitty).proposal, Setting::FontSize), 19.0);
}

// ---------------------------------------------------------------- numbers

#[test]
fn a_number_that_is_not_one_is_not_a_setting() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/alacritty/alacritty.toml",
        "[font]\nsize = nan\noffset = { x = 0, y = inf }\n[window]\nopacity = -inf\n",
    );
    let p = fixture.found(Source::Alacritty).proposal;
    assert!(p.values.is_empty(), "{:?}", p.values);
    for key in ["font.size", "font.offset.y", "window.opacity"] {
        assert!(skipped(&p, key, &Reason::UnsupportedValue), "{key}");
    }
}

#[test]
fn a_font_size_of_nothing_does_not_divide_by_it() {
    let fixture = Fixture::new();
    fixture.home(".config/ghostty/config", "font-size = 0\nadjust-cell-height = 2\n");
    let p = fixture.found(Source::Ghostty).proposal;
    assert_eq!(number(&p, Setting::FontSize), 8.0);
    assert!(number(&p, Setting::LineHeight).is_finite());

    let fixture = Fixture::new();
    fixture.home(".config/kitty/kitty.conf", "font_size 0\nmodify_font cell_height 2\n");
    let p = fixture.found(Source::Kitty).proposal;
    assert!(number(&p, Setting::LineHeight).is_finite());

    let fixture = Fixture::new();
    fixture.home(".config/alacritty/alacritty.toml", "[font]\nsize = 0\noffset = { x = 0, y = 2 }\n");
    assert!(number(&fixture.found(Source::Alacritty).proposal, Setting::LineHeight).is_finite());
}

#[test]
fn the_conversions_turn_a_number_that_is_not_one_into_the_default() {
    use crate::config::Settings;
    use crate::import::units::{finite, font_size, line_height, opacity};
    let defaults = Settings::default();
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(font_size(bad), defaults.font_size);
        assert_eq!(line_height(bad), defaults.line_height);
        assert_eq!(opacity(bad), defaults.opacity);
    }
    assert_eq!((font_size(1.0), line_height(9.0), opacity(0.0)), (8.0, 2.0, 0.3));
    assert_eq!(finite(f64::NAN), None);
    assert_eq!(finite(1e300), None, "finite as a double, too big for the float");
    assert_eq!(finite(1.5), Some(1.5));
}

#[test]
fn numbers_too_big_for_a_float_are_skipped_in_json_and_lua() {
    let fixture = Fixture::new();
    fixture.home(
        "Library/Application Support/Code/User/settings.json",
        r#"{ "terminal.integrated.fontSize": 1e300, "terminal.integrated.lineHeight": 1e300 }"#,
    );
    let p = fixture.found(Source::VsCode).proposal;
    assert!(p.values.is_empty());
    assert!(skipped(&p, "terminal.integrated.fontSize", &Reason::UnsupportedValue));

    let fixture = Fixture::new();
    fixture.home(".config/wezterm/wezterm.lua", "config.font_size = 1e999\nconfig.line_height = 1.2\n");
    let p = fixture.found(Source::WezTerm).proposal;
    assert!(skipped(&p, "font_size", &Reason::UnsupportedValue));
    assert!(p.get(Setting::FontSize).is_none());
    assert_eq!(number(&p, Setting::LineHeight), 1.2);
}

// ---------------------------------------------------------------- palettes

#[test]
fn colours_short_of_a_background_and_a_foreground_are_not_a_palette() {
    let partial = |source: Source, p: &crate::import::Proposal| {
        assert!(p.get(Setting::Theme).is_none(), "{source:?} proposed a palette");
        assert!(
            p.skipped.iter().any(|skip| skip.reason == Reason::PartialPalette),
            "{source:?}: {:?}",
            p.skipped
        );
    };

    let fixture = Fixture::new();
    fixture.home(".config/ghostty/config", "background = #101010\npalette = 1=#ff0000\n");
    partial(Source::Ghostty, &fixture.found(Source::Ghostty).proposal);

    let fixture = Fixture::new();
    fixture.home(".config/ghostty/themes/Half", "palette = 0=#000000\nforeground = #ffffff\n");
    fixture.home(".config/ghostty/config", "theme = Half\n");
    partial(Source::Ghostty, &fixture.found(Source::Ghostty).proposal);

    let fixture = Fixture::new();
    fixture.home(".config/kitty/kitty.conf", "foreground #ffffff\ncolor1 #ff0000\n");
    partial(Source::Kitty, &fixture.found(Source::Kitty).proposal);

    let fixture = Fixture::new();
    fixture.home(".config/alacritty/alacritty.toml", "[colors.normal]\nred = \"#ff0000\"\n");
    partial(Source::Alacritty, &fixture.found(Source::Alacritty).proposal);

    let fixture = Fixture::new();
    fixture.home(".config/wezterm/wezterm.lua", "config.colors = { foreground = '#ffffff' }\n");
    partial(Source::WezTerm, &fixture.found(Source::WezTerm).proposal);

    let fixture = Fixture::new();
    fixture.home(".warp/themes/half.yaml", "foreground: '#ffffff'\n");
    fixture.home(
        ".warp/settings.toml",
        "[appearance.themes]\ntheme = { custom = { name = \"Half\", path = \"half.yaml\" } }\n",
    );
    partial(Source::Warp, &fixture.found(Source::Warp).proposal);

    let fixture = Fixture::new();
    fixture.home(".config/zed/settings.json", r##"{ "theme": "Half" }"##);
    fixture.home(
        ".config/zed/themes/half.json",
        r##"{ "themes": [ { "name": "Half", "style": { "terminal.ansi.red": "#ff0000" } } ] }"##,
    );
    partial(Source::Zed, &fixture.found(Source::Zed).proposal);

    let fixture = Fixture::new();
    fixture.home(
        "Library/Application Support/Code/User/settings.json",
        r##"{ "workbench.colorCustomizations": { "terminal.foreground": "#ffffff" } }"##,
    );
    partial(Source::VsCode, &fixture.found(Source::VsCode).proposal);
}

#[test]
fn a_whole_palette_is_still_proposed() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/alacritty/alacritty.toml",
        "[colors.primary]\nbackground = \"#000000\"\nforeground = \"#ffffff\"\n",
    );
    let p = fixture.found(Source::Alacritty).proposal;
    assert_eq!(palette(&p, Setting::Theme).foreground, 0xffffff);
    assert!(!p.skipped.iter().any(|skip| skip.reason == Reason::PartialPalette));
}

// ---------------------------------------------------------------- paths

#[test]
fn ghostty_reads_no_include_or_theme_outside_the_home() {
    let fixture = Fixture::new();
    let secret = outside(&fixture, "secret.conf", "font-size = 31\n");
    let night = outside(&fixture, "night", "background = #000000\nforeground = #ffffff\n");
    let linked = outside(&fixture, "linked.conf", "font-size = 33\n");
    crate::platform::fs::symlink(&linked, fixture.env.home.join(".config-link.conf")).expect("symlink");
    fixture.home(
        ".config/ghostty/config",
        &format!(
            "font-size = 12\nconfig-file = {}\nconfig-file = ~/../outside/secret.conf\n\
             config-file = ~/.config-link.conf\ntheme = {}\n",
            secret.display(),
            night.display()
        ),
    );
    let found = left_alone(&fixture, Source::Ghostty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 12.0);
    assert!(found.proposal.get(Setting::Theme).is_none());
    assert!(!found.files.iter().any(|file| file.starts_with(fixture.root().join("outside"))));
    // Once for the name outside, once for the climb out, once for the link out, once for the theme.
    let count = found.proposal.skipped.iter().filter(|skip| skip.reason == Reason::OutsideHome).count();
    assert_eq!(count, 4);
}

#[test]
fn an_include_inside_the_home_is_still_read_even_through_a_link_inside_it() {
    let fixture = Fixture::new();
    let target = fixture.home("dotfiles/ghostty.conf", "font-size = 17\n");
    fixture
        .home(".config/ghostty/config", "config-file = link.conf\nconfig-file = ~/dotfiles/ghostty.conf\n");
    crate::platform::fs::symlink(&target, fixture.env.home.join(".config/ghostty/link.conf"))
        .expect("symlink");
    let found = fixture.found(Source::Ghostty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 17.0);
    assert!(!found.proposal.skipped.iter().any(|skip| skip.reason == Reason::OutsideHome));
}

#[test]
fn kitty_reads_no_include_or_glob_outside_the_home() {
    let fixture = Fixture::new();
    let secret = outside(&fixture, "secret.conf", "font_size 31\n");
    fixture.home(
        ".config/kitty/kitty.conf",
        &format!(
            "font_size 12\ninclude {}\nglobinclude {}/*.conf\n",
            secret.display(),
            secret.parent().expect("folder").display()
        ),
    );
    let found = left_alone(&fixture, Source::Kitty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 12.0);
    assert_eq!(found.proposal.skipped.iter().filter(|skip| skip.reason == Reason::OutsideHome).count(), 2);
}

#[test]
fn alacritty_imports_nothing_outside_the_home() {
    let fixture = Fixture::new();
    let secret = outside(&fixture, "secret.toml", "[font]\nsize = 31\n");
    fixture.home(
        ".config/alacritty/alacritty.toml",
        &format!("[general]\nimport = [\"{}\"]\n[font]\nsize = 12\n", secret.display()),
    );
    let found = left_alone(&fixture, Source::Alacritty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 12.0);
}

#[test]
fn a_warp_theme_outside_the_home_is_not_read() {
    let fixture = Fixture::new();
    let theme = outside(&fixture, "theme.yaml", "background: '#000000'\nforeground: '#ffffff'\n");
    fixture.home(
        ".warp/settings.toml",
        &format!(
            "[appearance.themes]\ntheme = {{ custom = {{ name = \"Out\", path = \"{}\" }} }}\n",
            theme.display()
        ),
    );
    let found = left_alone(&fixture, Source::Warp);
    assert!(found.proposal.get(Setting::Theme).is_none());
}

#[test]
fn volumes_are_outside_the_home_and_not_even_looked_at() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/ghostty/config",
        "font-size = 12\nconfig-file = /Volumes/Stale Share/ghostty.conf\ntheme = /Volumes/Stale Share/night\n",
    );
    let found = left_alone(&fixture, Source::Ghostty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 12.0);
}
