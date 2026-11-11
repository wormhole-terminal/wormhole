//! What the program's pages are made of (`WelcomeOp::State`): the look on offer and in use, what reaches
//! the person, the agents found. The lists are the ones the intro window's Look step offers.

use wormhole_core::config::Settings;
use wormhole_core::ipc::control::{LookFacts, NotifyFacts, PrimaryFact, ThemeFact, WelcomeState};
use wormhole_core::onboarding::LOOK_THEMES;
use wormhole_core::onboarding::intro::Handover;
use wormhole_core::storage::paths::ProfilePaths;
use wormhole_core::theme::fonts::FontSlot;
use wormhole_core::theme::registry::UserThemes;
use wormhole_core::theme::tokens::{Appearance, Color, ThemeTokens, builtins};

use super::{ImportedTheme, Run, wait};

/// The terminal colours offered as a primary, in the order the intro window lists them (green first).
const PRIMARY_ANSI: [usize; 5] = [2, 5, 1, 3, 6];

/// The themes on offer: core's list, filled up with others where a build lacks some, and always with the
/// theme in use among them.
pub(super) fn offered_themes(current: &str) -> Vec<&'static ThemeTokens> {
    let mut themes: Vec<&ThemeTokens> =
        LOOK_THEMES.iter().filter_map(|name| builtins().iter().find(|t| t.name == *name)).collect();
    for theme in builtins() {
        if themes.len() >= LOOK_THEMES.len() {
            break;
        }
        if !themes.iter().any(|t| t.name == theme.name) {
            themes.push(theme);
        }
    }
    if !themes.iter().any(|t| t.name == current)
        && let Some(theme) = builtins().iter().find(|t| t.name == current)
    {
        themes.pop();
        themes.push(theme);
    }
    themes
}

/// The theme's own primary and those of its terminal colours that differ from it, each with its place in
/// the 16.
pub(super) fn offered_primaries(base: &ThemeTokens) -> (Option<Color>, Vec<(usize, Color)>) {
    let theme_primary = base.get("primary").map(|c| c.with_alpha(1.0));
    let mut colors: Vec<(usize, Color)> = Vec::new();
    for ix in PRIMARY_ANSI {
        let color = base.terminal.ansi[ix].with_alpha(1.0);
        if Some(color.to_hex()) != theme_primary.map(|c| c.to_hex())
            && !colors.iter().any(|(_, c)| c.to_hex() == color.to_hex())
        {
            colors.push((ix, color));
        }
    }
    (theme_primary, colors)
}

/// The theme `settings.theme` names, resolved as the app does (a user's file, else a bundled one).
pub(super) fn base_tokens(profile: &ProfilePaths, value: &str) -> Option<ThemeTokens> {
    let themes = UserThemes::read(&profile.themes_dir());
    let user = themes.named(value).or_else(|| themes.by_path(value));
    match user {
        Some(theme) => themes.resolve(theme).ok(),
        None => builtins().iter().find(|t| t.name == value).cloned(),
    }
}

fn ansi_name(ix: usize) -> &'static str {
    ["Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White"][ix % 8]
}

pub(super) fn look(profile: &ProfilePaths, s: &Settings, imported: Option<&ImportedTheme>) -> LookFacts {
    let mut themes: Vec<ThemeFact> = offered_themes(&s.theme)
        .into_iter()
        .map(|t| ThemeFact { name: t.name.clone(), light: t.appearance == Appearance::Light })
        .collect();
    // The theme an import brought over stays on the list, in use or not.
    let mut in_use = s.theme.clone();
    if let Some(imported) = imported {
        themes.push(ThemeFact { name: imported.name.clone(), light: imported.light });
        if s.theme == imported.value {
            in_use.clone_from(&imported.name);
        }
    }
    let base = base_tokens(profile, &s.theme);
    let mut primaries = Vec::new();
    if let Some(base) = &base {
        let (theme_primary, others) = offered_primaries(base);
        if theme_primary.is_some() {
            primaries.push(PrimaryFact { name: "Theme".to_string(), hex: String::new(), ansi: None });
        }
        for (ix, color) in others {
            primaries.push(PrimaryFact {
                name: ansi_name(ix).to_string(),
                hex: color.to_hex(),
                ansi: u8::try_from(ix).ok(),
            });
        }
    }
    LookFacts {
        themes,
        theme: in_use,
        primaries,
        primary: s.primary().map(|color| color.to_hex()).unwrap_or_default(),
        font: s.slot_family(FontSlot::Terminal).to_string(),
        font_size: s.slot_size(FontSlot::Terminal),
        ligatures: s.font_ligatures,
    }
}

pub(super) fn state(profile: &ProfilePaths, s: &Settings, run: &Run, notify: NotifyFacts) -> WelcomeState {
    WelcomeState {
        wait_ms: wait(run).as_millis() as u64,
        reduce_motion: run.reduce_motion,
        handover: run.handover.key().into(),
        first_run: run.first_run,
        look: look(profile, s, run.imported.as_ref()),
        notify: Some(notify),
        agents: run.agents.clone(),
        // "How much to show" changes the window's own panel, which this app's window does not report yet: the
        // program shows that page as the tour does, read-only.
        shown: None,
        problem: run.problem.clone(),
    }
}

pub(super) fn empty_state() -> WelcomeState {
    WelcomeState {
        wait_ms: 0,
        reduce_motion: false,
        handover: Handover::default().key().into(),
        first_run: false,
        look: LookFacts {
            themes: Vec::new(),
            theme: String::new(),
            primaries: Vec::new(),
            primary: String::new(),
            font: String::new(),
            font_size: 0.0,
            ligatures: false,
        },
        notify: None,
        agents: None,
        shown: None,
        problem: None,
    }
}
