//! The rows of Appearance that are not a plain switch or menu: the primary and branch colours (Theme, one of
//! eight swatches, or Custom from the native colour well), the title row's parts and the icon theme.
//!
//! A colour row reads as its pick (`theme`, `primary` for the branch, a swatch id or `custom`) and a `#custom`
//! sub-value holding the hex of the custom colour. A swatch is stored as its hex, so a hand-edited file that
//! happens to hold a swatch's hex shows that swatch.

use wormhole_core::config::BranchColor;
use wormhole_core::config::Settings;
use wormhole_core::view::layout::title_parts::TitlePart;

/// The eight swatches: the window's id and the hex a pick stores. The same table as the window's.
const SWATCHES: [(&str, &str); 8] = [
    ("blue", "#0a84ff"),
    ("purple", "#bf5af2"),
    ("pink", "#ff375f"),
    ("red", "#ff453a"),
    ("orange", "#ff9f0a"),
    ("yellow", "#ffd60a"),
    ("green", "#30d158"),
    ("graphite", "#8e8e93"),
];

const THEME: &str = "theme";
const PRIMARY: &str = "primary";
const CUSTOM: &str = "custom";

fn swatch_named(hex: &str) -> Option<&'static str> {
    SWATCHES.iter().find(|(_, h)| h.eq_ignore_ascii_case(hex.trim())).map(|(name, _)| *name)
}

fn swatch_hex(name: &str) -> Option<&'static str> {
    SWATCHES.iter().find(|(n, _)| *n == name).map(|(_, hex)| *hex)
}

pub(super) fn is_colour_row(id: &str) -> bool {
    matches!(id, "primary-color" | "branch-color")
}

pub(super) fn read_colour(id: &str, s: &Settings) -> Vec<(&'static str, String)> {
    let (pick, custom) = if id == "primary-color" {
        let hex = s.primary_color.trim();
        let pick = if s.uses_theme_primary() { THEME } else { swatch_named(hex).unwrap_or(CUSTOM) };
        (pick, hex.to_string())
    } else {
        let hex = s.branch_color_custom.trim();
        let pick = match s.branch_color {
            BranchColor::Theme => THEME,
            BranchColor::Primary => PRIMARY,
            BranchColor::Custom => swatch_named(hex).unwrap_or(CUSTOM),
        };
        (pick, hex.to_string())
    };
    vec![("", pick.to_string()), ("#custom", custom)]
}

pub(super) fn write_colour(id: &str, suffix: &str, s: &mut Settings, value: &str) -> bool {
    let primary = id == "primary-color";
    match suffix {
        "#custom" => {
            if wormhole_core::theme::tokens::Color::from_hex(value.trim()).is_err() {
                return false;
            }
            if primary {
                s.choose_primary(value.trim().to_string());
            } else {
                s.choose_branch_color(value.trim().to_string());
            }
            true
        }
        "" => {
            match (primary, value) {
                (true, THEME) => s.use_theme_primary(),
                (false, THEME) => s.choose_branch_source(BranchColor::Theme),
                (false, PRIMARY) => s.choose_branch_source(BranchColor::Primary),
                (true, CUSTOM) => {
                    // Custom with no colour yet: the primary in use, else the first swatch.
                    let hex = s.primary_color.trim().to_string();
                    s.choose_primary(if hex.is_empty() { SWATCHES[0].1.into() } else { hex });
                }
                (false, CUSTOM) => {
                    let hex = s.branch_color_custom.trim().to_string();
                    s.choose_branch_color(if hex.is_empty() { SWATCHES[0].1.into() } else { hex });
                }
                (true, name) => match swatch_hex(name) {
                    Some(hex) => s.choose_primary(hex.into()),
                    None => return false,
                },
                (false, name) => match swatch_hex(name) {
                    Some(hex) => s.choose_branch_color(hex.into()),
                    None => return false,
                },
            }
            true
        }
        _ => false,
    }
}

/// The title row's parts: the id the window uses in a sub-value (`#folder`) and the part.
const PARTS: [(&str, TitlePart); 10] = [
    ("folder", TitlePart::Folder),
    ("branch", TitlePart::Branch),
    ("branch_status", TitlePart::BranchStatus),
    ("terminal", TitlePart::Terminal),
    ("terminal_status", TitlePart::TerminalStatus),
    ("agent_gauge", TitlePart::AgentGauge),
    ("ports", TitlePart::Ports),
    ("pull_request", TitlePart::PullRequest),
    ("open_in", TitlePart::OpenIn),
    ("splits", TitlePart::Splits),
];

pub(super) fn read_title(s: &Settings) -> Vec<(&'static str, String)> {
    PARTS
        .iter()
        .map(|(id, part)| (suffix_of(id), if s.title_shows(*part) { "on" } else { "off" }.to_string()))
        .collect()
}

fn suffix_of(id: &str) -> &'static str {
    match id {
        "folder" => "#folder",
        "branch" => "#branch",
        "branch_status" => "#branch_status",
        "terminal" => "#terminal",
        "terminal_status" => "#terminal_status",
        "agent_gauge" => "#agent_gauge",
        "ports" => "#ports",
        "pull_request" => "#pull_request",
        "open_in" => "#open_in",
        _ => "#splits",
    }
}

pub(super) fn write_title(suffix: &str, s: &mut Settings, value: &str) -> bool {
    let Some((_, part)) = PARTS.iter().find(|(id, _)| Some(*id) == suffix.strip_prefix('#')) else {
        return false;
    };
    s.show_title_part(*part, value == "on");
    true
}

pub(super) fn read_icon_theme(s: &Settings) -> Vec<(&'static str, String)> {
    vec![("", s.icon_theme.clone())]
}

/// The installed icon themes; one no longer installed is listed as written until another is chosen.
pub(super) fn icon_theme_options(s: &Settings) -> Vec<String> {
    let mut names = crate::icons::available();
    if !names.contains(&s.icon_theme) {
        names.push(s.icon_theme.clone());
    }
    names
}

pub(super) fn write_icon_theme(s: &mut Settings, value: &str) -> bool {
    s.icon_theme = value.to_string();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swatch_round_trips_and_custom_keeps_its_hex() {
        let mut s = Settings::default();
        assert_eq!(read_colour("primary-color", &s)[0].1, "theme");
        assert!(write_colour("primary-color", "", &mut s, "orange"));
        assert_eq!(read_colour("primary-color", &s)[0].1, "orange");
        assert!(write_colour("primary-color", "#custom", &mut s, "#123456"));
        assert_eq!(read_colour("primary-color", &s)[0].1, "custom");
        assert!(write_colour("primary-color", "", &mut s, "theme"));
        assert!(s.uses_theme_primary());
        assert!(!write_colour("primary-color", "#custom", &mut s, "not a colour"));
    }

    #[test]
    fn the_branch_can_follow_the_primary() {
        let mut s = Settings::default();
        assert!(write_colour("branch-color", "", &mut s, "primary"));
        assert_eq!(read_colour("branch-color", &s)[0].1, "primary");
    }

    #[test]
    fn a_title_part_is_hidden_and_shown_by_id() {
        let mut s = Settings::default();
        assert!(write_title("#ports", &mut s, "off"));
        assert!(!s.title_shows(TitlePart::Ports));
        assert!(write_title("#ports", &mut s, "on"));
        assert!(s.title_shows(TitlePart::Ports));
        assert!(!write_title("#nonsense", &mut s, "on"));
    }
}
