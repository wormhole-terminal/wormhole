//! The command palette's actions as the table has them: which actions it offers for the
//! window in front (those its menus would offer), what it calls them (sentence case, a toggle by
//! what it will do: "Hide sidebar", and a window by what opening it is: "Open Theme Studio"),
//! the order and groups it lists them in (what people reach for most first),
//! and the few it suggests before anything is typed. Their keys are the table's, so the palette
//! teaches the keys the menus show.

use super::menus::MenuState;
use super::{ActionId, Entry, Stage, entries, entry};
use crate::keyspec::KeySpec;

/// The groups of the Actions scope (Terminal, View, the project's own, App).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Terminal,
    View,
    App,
}

impl Section {
    pub fn title(self) -> &'static str {
        match self {
            Section::Terminal => "Terminal",
            Section::View => "View",
            Section::App => "App",
        }
    }
}

/// An action as the palette lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub id: ActionId,
    pub label: String,
    pub key: Option<KeySpec>,
    pub section: Section,
}

/// What the palette suggests with nothing typed, after the recent results: the actions that make
/// sense in most windows, each with its key (the empty query).
pub const SUGGESTED: [ActionId; 6] = [
    ActionId::NewTerminal,
    ActionId::OpenFolder,
    ActionId::ShowChanges,
    ActionId::ToggleRightPanel,
    ActionId::StashPrompt,
    ActionId::KeyboardShortcuts,
];

/// The actions the palette lists first, in this order (the Terminal and View groups, then the
/// suggestions): what people reach for most. The rest follow in the table's order.
const COMMON: [ActionId; 13] = [
    ActionId::NewTerminal,
    ActionId::CloseTerminal,
    ActionId::OpenFolder,
    ActionId::NewWindow,
    ActionId::ToggleSidebar,
    ActionId::ShowChanges,
    ActionId::ShowHistory,
    ActionId::ToggleRightPanel,
    ActionId::ShowNotes,
    ActionId::StashPrompt,
    ActionId::KeyboardShortcuts,
    ActionId::Settings,
    ActionId::ThemeStudio,
];

/// What the palette calls an action whose window it opens, where the menu's title is the window's
/// name ("Open Theme Studio").
const OPENS: &[(ActionId, &str)] = &[(ActionId::ThemeStudio, "Open Theme Studio")];

/// Words that keep their capital in a sentence-case label: proper names (the panel's tabs are
/// not: "Show changes", as the menu's "Changes" is only capitalized as a title).
const NAMES: &[&str] = &["Finder", "Wormhole", "Studio"];

/// What the palette calls an action: the menu's title in sentence case; a toggle's title for
/// what it will do now (`on`: its thing is shown), anything else by its name ("Show changes").
pub fn label(entry: &Entry, on: bool) -> String {
    label_in(entry, on, false)
}

/// [`label`] for a window whose selected terminal is in a split: ⌘W closes its pane there.
fn label_in(entry: &Entry, on: bool, split: bool) -> String {
    if let Some((_, label)) = OPENS.iter().find(|(id, _)| *id == entry.id) {
        return label.to_string();
    }
    if entry.id == ActionId::CloseTerminal && split {
        return "Close pane".to_string();
    }
    let title = if entry.title_off.is_some() { entry.title_for(on) } else { entry.name() };
    let mut words = title.split(' ');
    let mut out = words.next().unwrap_or_default().to_string();
    for word in words {
        out.push(' ');
        if NAMES.contains(&word) {
            out.push_str(word);
        } else {
            out.push_str(&word.to_lowercase());
        }
    }
    out
}

/// The group an action is listed in.
pub fn section(id: ActionId) -> Section {
    use ActionId::*;
    match id {
        ToggleSidebar | ToggleRightPanel | ToggleStatusBar | ToggleMinimap | ShowNotes | ShowChanges
        | ShowHistory | ShowProject | ShowUsage | CompactRows | ToggleTerminalTabs | FocusTerminalOrPanel
        | FocusNextArea | FocusPreviousArea | Bigger | Smaller | ActualSize | FullScreen
        | KeyboardShortcuts | Minimize | Zoom | BringAllToFront | LayoutColumns | LayoutRows | LayoutGrid
        | LayoutSingle => Section::View,
        About | CheckForUpdates | Settings | ThemeStudio | Hide | HideOthers | ShowAll | Quit | ShowLogs
        | Inspector | Help | CommandPalette | ShowIntro | PlayIntroEmerge | PlayIntroVortex
        | PlayIntroCollapse | PlayIntroFtl | PlayIntroAurora | PlayIntroTentacles | PlayIntroVoyage
        | PlayWelcomeCursor | PlayWelcomePortal | PlayWelcomeLogo => Section::App,
        _ => Section::Terminal,
    }
}

/// Every action the palette offers for the window `state` describes, the most used first
/// ([`COMMON`], then the table's order): the live ones it lists (not the editing keys or the
/// numbered ones), and of those only what the window can do now (a menu item it would grey out
/// isn't offered).
pub fn actions(state: &MenuState) -> Vec<Action> {
    let mut actions: Vec<Action> = entries()
        .iter()
        .filter(|e| e.stage == Stage::Now && e.palette && (!e.debug_only || state.debug))
        .filter(|e| !state.disabled.contains(&e.id))
        .map(|e| Action {
            id: e.id,
            label: label_in(e, !state.off.contains(&e.id), state.split),
            key: e.key().cloned(),
            section: section(e.id),
        })
        .collect();
    actions.sort_by_key(|a| COMMON.iter().position(|id| *id == a.id).unwrap_or(COMMON.len()));
    actions
}

/// [`SUGGESTED`] as `actions` offers them (one the window can't do is left out).
pub fn suggested(actions: &[Action]) -> Vec<Action> {
    SUGGESTED.iter().filter_map(|id| actions.iter().find(|a| a.id == *id).cloned()).collect()
}

/// `id`'s label for a window whose toggles are as `state` says.
pub fn label_of(id: ActionId, state: &MenuState) -> Option<String> {
    entry(id).map(|e| label_in(e, !state.off.contains(&id), state.split))
}

#[cfg(test)]
mod tests {
    use super::{Section, actions, label_of, section, suggested};
    use crate::keymap::ActionId;
    use crate::keymap::menus::MenuState;

    fn labels(state: &MenuState) -> Vec<String> {
        actions(state).into_iter().map(|a| a.label).collect()
    }

    #[test]
    fn labels_are_sentence_case_and_toggles_say_what_they_will_do() {
        let state =
            MenuState { off: vec![ActionId::ToggleStatusBar, ActionId::ZoomPane], ..MenuState::default() };
        let found = labels(&state);
        for label in [
            "New terminal",
            "Open folder…",
            "Close terminal",
            "Hide sidebar",
            "Hide right panel",
            "Show status bar",
            "Show changes",
            "Show history",
            "Show notes",
            "Show project",
            "Compact rows",
            "Focus terminal / panel",
            "Keyboard shortcuts",
            "Show logs in Finder",
            "Open Theme Studio",
            "Split right",
            "Zoom pane",
            "Move pane out of split",
            "Select pane on the left",
            "Layout: 2 columns",
            "Layout: 2×2 grid",
        ] {
            assert!(found.iter().any(|l| l == label), "{label:?} in {found:?}");
        }
        // In a split ⌘W closes the focused pane, and a zoomed pane is unzoomed.
        let split = MenuState { split: true, ..MenuState::default() };
        assert_eq!(label_of(ActionId::CloseTerminal, &split).as_deref(), Some("Close pane"));
        assert_eq!(label_of(ActionId::CloseTerminal, &state).as_deref(), Some("Close terminal"));
        assert_eq!(label_of(ActionId::ZoomPane, &state).as_deref(), Some("Zoom pane"));
        let zoomed = MenuState::default();
        assert_eq!(label_of(ActionId::ZoomPane, &zoomed).as_deref(), Some("Unzoom pane"));
        assert_eq!(label_of(ActionId::ToggleSidebar, &state).as_deref(), Some("Hide sidebar"));
        let hidden = MenuState { off: vec![ActionId::ToggleSidebar], ..MenuState::default() };
        assert_eq!(label_of(ActionId::ToggleSidebar, &hidden).as_deref(), Some("Show sidebar"));
    }

    #[test]
    fn the_palette_offers_what_the_window_can_do_with_the_tables_keys() {
        let all = actions(&MenuState::default());
        let ids: Vec<ActionId> = all.iter().map(|a| a.id).collect();
        // Not itself, nor the numbered or editing keys, nor what is the system's.
        for id in [ActionId::CommandPalette, ActionId::Terminal(1), ActionId::Copy, ActionId::Help] {
            assert!(!ids.contains(&id), "{id:?}");
        }
        assert!(!ids.contains(&ActionId::Inspector), "debug builds only");
        let debug = actions(&MenuState { debug: true, ..MenuState::default() });
        assert!(!debug.iter().any(|a| a.id == ActionId::Inspector), "not a palette action at all");
        let shortcuts = all.iter().find(|a| a.id == ActionId::KeyboardShortcuts).expect("offered");
        assert_eq!(shortcuts.key.as_ref().map(|k| k.glyphs()).as_deref(), Some("⌥⌘/"));
        // A menu item the window would grey out isn't offered.
        let no_folder = actions(&MenuState { disabled: vec![ActionId::ShowProject], ..MenuState::default() });
        assert!(!no_folder.iter().any(|a| a.id == ActionId::ShowProject));
    }

    #[test]
    fn suggestions_drop_what_the_window_cant_do() {
        let all = actions(&MenuState::default());
        let shown: Vec<String> = suggested(&all).into_iter().map(|a| a.label).collect();
        assert_eq!(
            shown,
            [
                "New terminal",
                "Open folder…",
                "Show changes",
                "Hide right panel",
                "Stash prompt",
                "Keyboard shortcuts"
            ]
        );
        let no_changes =
            actions(&MenuState { disabled: vec![ActionId::ShowChanges], ..MenuState::default() });
        assert_eq!(suggested(&no_changes).len(), 5);
    }

    #[test]
    fn every_label_is_sentence_case() {
        // Proper names keep their capital: the app, the Mac's Finder, the Theme Studio's window.
        const PROPER: &[&str] = &["Wormhole", "Finder", "Theme", "Studio"];
        let everything_on = MenuState { debug: true, ..MenuState::default() };
        let everything_off = MenuState {
            debug: true,
            off: actions(&everything_on).into_iter().map(|a| a.id).collect(),
            ..MenuState::default()
        };
        for action in actions(&everything_on).into_iter().chain(actions(&everything_off)) {
            for word in action.label.split(' ').skip(1) {
                assert!(
                    PROPER.contains(&word) || !word.starts_with(char::is_uppercase),
                    "{:?} is not sentence case",
                    action.label
                );
            }
        }
    }

    /// The palette lists the actions people use most first ("New terminal, Close terminal";
    /// "Hide sidebar, Show changes, Show history"; there is no Rename terminal: a terminal
    /// is named by what runs in it); the order is fixed, so it never shifts under the fingers.
    #[test]
    fn actions_come_in_order_of_use() {
        let all = actions(&MenuState::default());
        let first = |section: Section, n: usize| -> Vec<ActionId> {
            all.iter().filter(|a| a.section == section).take(n).map(|a| a.id).collect()
        };
        use ActionId::*;
        assert_eq!(first(Section::Terminal, 3), [NewTerminal, CloseTerminal, OpenFolder]);
        assert_eq!(first(Section::View, 3), [ToggleSidebar, ShowChanges, ShowHistory]);
        // What isn't listed first keeps the table's order after it.
        let rest: Vec<ActionId> =
            all.iter().filter(|a| a.section == Section::Terminal).map(|a| a.id).collect();
        let pin = rest.iter().position(|id| *id == PinTerminal);
        let reveal = rest.iter().position(|id| *id == RevealInFinder);
        assert!(pin < reveal && pin > Some(3), "{rest:?}");
    }

    #[test]
    fn every_offered_action_has_a_group() {
        assert_eq!(section(ActionId::NewTerminal), Section::Terminal);
        assert_eq!(section(ActionId::ToggleSidebar), Section::View);
        assert_eq!(section(ActionId::Settings), Section::App);
        assert_eq!(Section::View.title(), "View");
    }
}
