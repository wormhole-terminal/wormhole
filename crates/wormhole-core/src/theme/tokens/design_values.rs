//! The values the design was drawn with, as the rules
//! derive them. The golden files under `tests/golden/tokens/` pin every value;
//! these say which ones the design depends on, and why a change to them matters.

use super::test_support::{builtin, hex, ratio, resolve_str};
use super::{Appearance, Color, ThemeTokens, builtin_tokens};

/// Equal within one step per 8-bit channel: the values were computed in another tool.
fn near(t: &ThemeTokens, name: &str, want: &str) {
    let got = t.get(name).unwrap_or_else(|| panic!("{}: no {name}", t.name));
    let close = got.to_bytes().iter().zip(hex(want).to_bytes()).all(|(a, b)| a.abs_diff(b) <= 1);
    assert!(close, "{}: {name} is {}, the design has {want}", t.name, got.to_hex());
}

/// Contrast of `fg` on `bg` composited over `underlay` (over the window), as
/// `contrast_warnings` measures it.
fn ratio_on(t: &ThemeTokens, fg: &str, bg: &str, underlay: &str) -> f32 {
    let bg = t.opaque(bg, underlay).unwrap();
    Color::contrast_ratio(t.get(fg).unwrap().over(bg), bg)
}

#[test]
fn tokyo_night_selection_matches_the_design() {
    let t = builtin("Tokyo Night");
    near(&t, "sidebar.row.selected", "#232a3d");
    near(&t, "sidebar.row.selected.text", "#7aa2f7");
    near(&t, "sidebar.row.selected.muted", "#8991b1");
    near(&t, "list.selected.muted", "#9aa1c2");
    // Tokyo Night's own blue already reads on its tint, so the name keeps the primary.
    assert_eq!(t.get("sidebar.row.selected.text"), t.get("primary"));
}

/// Catppuccin Latte. One value differs from the design on purpose: the design draws
/// `text.muted` #595b6d, which is what reaches 4.5:1 on the selected row; it was settled
/// that muted text is judged on bands only and the selected row has its own
/// `sidebar.row.selected.muted` (which is that #595b6d).
#[test]
fn catppuccin_latte_matches_the_design() {
    let t = builtin("Catppuccin Latte");
    near(&t, "sidebar.row.selected.text", "#024edc");
    near(&t, "sidebar.row.selected.muted", "#595b6d");
    near(&t, "text.muted", "#606273");
    near(&t, "text.faint", "#7c7e8b");
    near(&t, "agent.attention.text", "#855000");
    near(&t, "agent.done.text", "#196b00");
    near(&t, "git.branch", "#883678");
    assert!(ratio(&t, "checkbox.border", "surface.panel") >= 3.0);
    assert!(ratio_on(&t, "text.muted", "band", "sidebar.background") >= 4.5);
    assert!(ratio_on(&t, "text.faint", "state.hover", "surface.panel") >= 3.0);
    assert!(ratio_on(&t, "git.branch", "git.branch.background", "band.solid") >= 4.5);
}

/// GitHub Light. Its ANSI yellow (#4d2d00) is darker than its muted text, so
/// the warning takes the amber rule: #ba8300, the value given for that rule. (The
/// other suggestion, a per-theme #9a6700, has no slot: built-in themes are plain
/// palettes.) The text variant is #845c00 rather than #875a00, which was
/// derived from #9a6700.
#[test]
fn github_light_matches_the_design() {
    let t = builtin("GitHub Light");
    near(&t, "status.warning", "#ba8300");
    near(&t, "status.warning.text", "#845c00");
    near(&t, "agent.attention.text", "#845c00");
    near(&t, "sidebar.row.selected.text", "#005fca");
    near(&t, "git.branch", "#6d47b2");
    assert!(ratio(&t, "checkbox.border", "surface.panel") >= 3.0);
}

#[test]
fn only_a_light_palette_with_a_dark_yellow_gets_the_amber_warning() {
    for t in builtin_tokens() {
        let from_yellow = resolve_str(&format!(
            r#"{{"extends": {:?}, "ui": {{"status.warning": "readable(terminal.ansi.3, surface.window, surface.panel, 3)"}}}}"#,
            t.name
        ))
        .unwrap();
        let amber = ["GitHub Light", "Wormhole Light", "Crimson Light"].contains(&t.name.as_str());
        assert_eq!(t.get("status.warning") != from_yellow.get("status.warning"), amber, "{}", t.name);
    }
    // Solarized Dark's yellow is darker than its muted text too, but on a dark
    // background it still reads as a warning.
    let solarized = builtin("Solarized Dark");
    assert_eq!(solarized.appearance, Appearance::Dark);
    assert!(solarized.terminal.ansi[3].luminance() < solarized.get("text.muted").unwrap().luminance());
    // A theme's own warning wins, and so does one that follows another token.
    let own = resolve_str(r##"{"extends": "GitHub Light", "ui": {"status.warning": "#9a6700"}}"##).unwrap();
    near(&own, "status.warning", "#9a6700");
    let follows =
        resolve_str(r##"{"extends": "GitHub Light", "ui": {"status.warning": "primary"}}"##).unwrap();
    assert_eq!(follows.get("status.warning"), follows.get("primary"));
    // A child of GitHub Light inherits the amber without writing it down.
    let child = resolve_str(r##"{"extends": "GitHub Light", "ui": {"primary": "#8250df"}}"##).unwrap();
    assert_eq!(child.get("status.warning"), builtin("GitHub Light").get("status.warning"));
    let saved = child.to_theme_file(Some(&builtin("GitHub Light")));
    assert!(!saved.ui.contains_key("status.warning"), "{:?}", saved.ui);
}

/// The right panel's counts and letters on a row and a selected row, and code on
/// the diff's line tints.
#[test]
fn right_panel_and_diff_text_read_like_the_design() {
    for t in builtin_tokens() {
        let n = &t.name;
        for fg in
            ["diff.added", "diff.removed", "status.success.text", "status.warning.text", "status.danger.text"]
        {
            assert!(ratio(&t, fg, "surface.panel") >= 4.5, "{n}: {fg} on the panel");
            let selected = ratio_on(&t, fg, "state.selected", "surface.panel");
            assert!(selected >= 4.5, "{n}: {fg} on a selected row is {selected:.2}");
        }
        for bg in ["diff.added.background", "diff.removed.background"] {
            for (key, style) in &t.syntax {
                let min =
                    if key.starts_with("comment") || key.starts_with("punctuation") { 3.0 } else { 4.5 };
                let on = Color::contrast_ratio(style.color, t.get(bg).unwrap());
                assert!(on >= min, "{n}: syntax.{key} on {bg} is {on:.2}");
            }
        }
    }
}

#[test]
fn bands_and_tints_follow_their_rules() {
    for t in builtin_tokens() {
        let get = |name: &str| t.get(name).unwrap();
        let n = &t.name;
        assert_eq!(get("band.solid"), get("band").over(get("sidebar.background")), "{n}");
        assert_eq!(get("band.solid").a, 1.0, "{n}");
        assert!(get("state.hover").a < get("band.hover").a && get("band.hover").a < get("state.active").a);
        assert_eq!(get("git.branch.background"), get("git.branch").with_alpha(0.14), "{n}");
        assert_eq!(get("agent.attention.ring"), get("agent.attention").with_alpha(0.28), "{n}");
        assert_eq!(get("agent.attention.tint"), get("agent.attention").with_alpha(0.12), "{n}");
        assert_eq!(get("chip.tint"), get("primary").with_alpha(0.16), "{n}");
        for kind in ["info", "warning", "danger", "success"] {
            let status = get(&format!("status.{kind}"));
            assert_eq!(get(&format!("banner.{kind}.background")), status.with_alpha(0.10), "{n}");
            assert_eq!(get(&format!("banner.{kind}.border")), status.with_alpha(0.35), "{n}");
        }
        assert_eq!(get("switch.knob"), Color::WHITE, "{n}");
        assert_eq!(get("resize.handle.active"), get("primary"), "{n}");
        assert_eq!(get("checkbox.border"), get("text.faint"), "{n}");
        assert_eq!(t.metrics.focus_ring_width, 2.0, "{n}");
    }
}
