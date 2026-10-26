//! The app's actions as GPUI actions and their keys, bound from core's `keymap` table. The menus
//! (`app::menus`) and the tooltips read the same table, so a key is written down once. Who handles
//! them: the app-wide ones in `app::handlers`, a window's terminals and panels in
//! `window::window_actions`.
//!
//! Below the window and the shared state, which bind and handle them. The actions' names come
//! from the namespaces given to `actions!` (`workspace`, `app`), not from this module's path, so
//! the keymap and the menus are unchanged by where the types live.

use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::component::input;
use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId;
use wormhole_core::keys::keymap::{self, ActionId, Scope, Stage};

use crate::terminal_view::{
    ClearBuffer, CopyCommand, CopyOutput, NextPrompt, PreviousPrompt, RerunCommand, SelectOutput,
};

actions!(
    workspace,
    [
        CheckForUpdates,
        NewWindow,
        CloseWindow,
        NewSession,
        NewTerminalInWorktree,
        NewTerminalNoGroup,
        PinTerminal,
        RevealInFinder,
        CopyPath,
        CloseTerminal,
        NextTerminal,
        PreviousTerminal,
        NextGroup,
        PreviousGroup,
        SplitRight,
        SplitDown,
        ZoomPane,
        LeaveSplit,
        FocusPaneLeft,
        FocusPaneRight,
        FocusPaneUp,
        FocusPaneDown,
        LayoutColumns,
        LayoutRows,
        LayoutGrid,
        LayoutSingle,
        ToggleSidebar,
        ToggleRightPanel,
        ShowNotes,
        ShowChanges,
        ShowHistory,
        ShowProject,
        ShowUsage,
        ToggleTerminalTabs,
        FocusTerminalOrPanel,
        FocusNextArea,
        FocusPreviousArea,
        Bigger,
        Smaller,
        ActualSize,
        NewNote,
        StashPrompt,
        Find,
        FindNext,
        FindPrevious,
        UseSelectionForFind,
    ]
);

actions!(
    app,
    [
        About,
        OpenFolder,
        GoToWaiting,
        GoToFinished,
        OpenSettings,
        ToggleThemeStudio,
        ToggleStatusBar,
        ToggleMinimap,
        Hide,
        HideOthers,
        ShowAll,
        Minimize,
        Zoom,
        ToggleFullScreen,
        BringAllToFront,
        ShowLogs,
        Licenses,
        ShowIntro,
        PlayIntroEmerge,
        PlayIntroVortex,
        PlayIntroCollapse,
        PlayIntroFtl,
        PlayIntroAurora,
        PlayIntroTentacles,
        PlayIntroVoyage,
        PlayWelcomeCursor,
        PlayWelcomePortal,
        PlayWelcomeLogo,
        ToggleCommandPalette,
        ShowKeyboardShortcuts,
        NewProfile,
        ManageProfiles,
        /// The greyed line in an empty submenu ("No Recent Folders"): nothing handles it, so
        /// AppKit keeps it disabled whatever is in front.
        Placeholder,
    ]
);

/// ⌘1–9: the sidebar's terminal with that number.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = workspace, no_json)]
pub struct GoToTerminal(pub u8);

/// ⌃⌥1–9: the group with that number, as the tab bar's switcher lists them.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = workspace, no_json)]
pub struct GoToGroup(pub u8);

/// Shell → Open Recent: a recent folder.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = app, no_json)]
pub struct OpenRecent(pub PathBuf);

/// Terminal → Project Actions: the front terminal's project action with this id.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = workspace, no_json)]
pub struct RunProjectAction(pub SharedString);

/// Shell → Profile → a profile: Wormhole goes to it, every window with it. Also the palette's
/// "Switch to Work". The text is the profile's id.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = app, no_json)]
pub struct SwitchProfile(pub SharedString);

/// Shell → Profile → Open Window in → a profile (and the palette's "Open window in Work").
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = app, no_json)]
pub struct OpenWindowInProfile(pub SharedString);

/// Shell → Profile → Move Window to → a profile (and the palette's "Move this window to Work").
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = app, no_json)]
pub struct MoveWindowToProfile(pub SharedString);

/// A link in a window that opens Settings at a page (the first run's "Coding agents settings"): the
/// window dispatches it and the app root, which owns the Settings window, handles it.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = app, no_json)]
pub struct OpenSettingsAt(pub SectionId);

/// A link in a window that opens Settings at one setting's row (the title row's "Customize title
/// row…"), by its id in core's settings table: dispatched like [`OpenSettingsAt`].
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = app, no_json)]
pub struct RevealSetting(pub &'static str);

/// Shell → New Terminal With: a new terminal running the shell with this key.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = workspace, no_json)]
pub struct NewTerminalWith(pub SharedString);

/// The inspector is in debug builds only (or with the `inspector` feature).
pub(crate) const DEBUG: bool = cfg!(any(feature = "inspector", debug_assertions));

/// The GPUI action of a table row. `None` for what nothing binds yet (later phases, the system's
/// Help search, the keys kept for Terminal.app's meanings) and for the keys a view reads from
/// the table itself (the terminal's line keys, a list's move keys, the Theme Studio's save keys).
pub(crate) fn gpui_action(id: ActionId) -> Option<Box<dyn Action>> {
    use ActionId as A;
    Some(match id {
        A::NewWindow => Box::new(NewWindow),
        A::NewTerminal => Box::new(NewSession),
        A::NewTerminalInWorktree => Box::new(NewTerminalInWorktree),
        A::NewTerminalNoGroup => Box::new(NewTerminalNoGroup),
        A::OpenFolder => Box::new(OpenFolder),
        A::PinTerminal => Box::new(PinTerminal),
        A::RevealInFinder => Box::new(RevealInFinder),
        A::CopyPath => Box::new(CopyPath),
        A::CloseTerminal => Box::new(CloseTerminal),
        A::CloseWindow => Box::new(CloseWindow),
        A::NextTerminal => Box::new(NextTerminal),
        A::PreviousTerminal => Box::new(PreviousTerminal),
        A::Terminal(n) => Box::new(GoToTerminal(n)),
        A::NextGroup => Box::new(NextGroup),
        A::PreviousGroup => Box::new(PreviousGroup),
        A::Group(n) => Box::new(GoToGroup(n)),
        A::SplitRight => Box::new(SplitRight),
        A::SplitDown => Box::new(SplitDown),
        A::ZoomPane => Box::new(ZoomPane),
        A::LeaveSplit => Box::new(LeaveSplit),
        A::FocusPaneLeft => Box::new(FocusPaneLeft),
        A::FocusPaneRight => Box::new(FocusPaneRight),
        A::FocusPaneUp => Box::new(FocusPaneUp),
        A::FocusPaneDown => Box::new(FocusPaneDown),
        A::LayoutColumns => Box::new(LayoutColumns),
        A::LayoutRows => Box::new(LayoutRows),
        A::LayoutGrid => Box::new(LayoutGrid),
        A::LayoutSingle => Box::new(LayoutSingle),
        A::Minimize => Box::new(Minimize),
        A::Zoom => Box::new(Zoom),
        A::BringAllToFront => Box::new(BringAllToFront),
        A::ToggleSidebar => Box::new(ToggleSidebar),
        A::ToggleRightPanel => Box::new(ToggleRightPanel),
        A::ToggleStatusBar => Box::new(ToggleStatusBar),
        A::ToggleMinimap => Box::new(ToggleMinimap),
        A::ShowNotes => Box::new(ShowNotes),
        A::ShowChanges => Box::new(ShowChanges),
        A::ShowHistory => Box::new(ShowHistory),
        A::ShowProject => Box::new(ShowProject),
        A::ShowUsage => Box::new(ShowUsage),
        A::ToggleTerminalTabs => Box::new(ToggleTerminalTabs),
        A::FocusTerminalOrPanel => Box::new(FocusTerminalOrPanel),
        A::FocusNextArea => Box::new(FocusNextArea),
        A::FocusPreviousArea => Box::new(FocusPreviousArea),
        A::Bigger => Box::new(Bigger),
        A::Smaller => Box::new(Smaller),
        A::ActualSize => Box::new(ActualSize),
        A::FullScreen => Box::new(ToggleFullScreen),
        A::Undo => Box::new(input::Undo),
        A::Redo => Box::new(input::Redo),
        A::Cut => Box::new(input::Cut),
        A::Copy => Box::new(input::Copy),
        A::Paste => Box::new(input::Paste),
        A::SelectAll => Box::new(input::SelectAll),
        A::ClearBuffer => Box::new(ClearBuffer),
        A::PreviousPrompt => Box::new(PreviousPrompt),
        A::NextPrompt => Box::new(NextPrompt),
        A::SelectOutput => Box::new(SelectOutput),
        A::CopyOutput => Box::new(CopyOutput),
        A::CopyCommand => Box::new(CopyCommand),
        A::RerunCommand => Box::new(RerunCommand),
        A::Find => Box::new(Find),
        A::FindNext => Box::new(FindNext),
        A::FindPrevious => Box::new(FindPrevious),
        A::UseSelectionForFind => Box::new(UseSelectionForFind),
        A::NewNote => Box::new(NewNote),
        A::StashPrompt => Box::new(StashPrompt),
        A::About => Box::new(About),
        A::CheckForUpdates => Box::new(CheckForUpdates),
        A::Settings => Box::new(OpenSettings),
        A::ThemeStudio => Box::new(ToggleThemeStudio),
        A::Hide => Box::new(Hide),
        A::HideOthers => Box::new(HideOthers),
        A::ShowAll => Box::new(ShowAll),
        A::Quit => Box::new(crate::Quit),
        A::QuitAndEndAll => Box::new(crate::QuitAndEndAll),
        A::ShowLogs => Box::new(ShowLogs),
        A::Licenses => Box::new(Licenses),
        A::ShowIntro => Box::new(ShowIntro),
        A::PlayIntroEmerge => Box::new(PlayIntroEmerge),
        A::PlayIntroVortex => Box::new(PlayIntroVortex),
        A::PlayIntroCollapse => Box::new(PlayIntroCollapse),
        A::PlayIntroFtl => Box::new(PlayIntroFtl),
        A::PlayIntroAurora => Box::new(PlayIntroAurora),
        A::PlayIntroTentacles => Box::new(PlayIntroTentacles),
        A::PlayIntroVoyage => Box::new(PlayIntroVoyage),
        A::PlayWelcomeCursor => Box::new(PlayWelcomeCursor),
        A::PlayWelcomePortal => Box::new(PlayWelcomePortal),
        A::PlayWelcomeLogo => Box::new(PlayWelcomeLogo),
        #[cfg(any(feature = "inspector", debug_assertions))]
        A::Inspector => Box::new(crate::inspector::Inspect),
        #[cfg(not(any(feature = "inspector", debug_assertions)))]
        A::Inspector => return None,
        A::GoToWaiting => Box::new(GoToWaiting),
        A::GoToFinished => Box::new(GoToFinished),
        A::CommandPalette => Box::new(ToggleCommandPalette),
        A::KeyboardShortcuts => Box::new(ShowKeyboardShortcuts),
        A::NewProfile => Box::new(NewProfile),
        A::ManageProfiles => Box::new(ManageProfiles),
        // These three act on the profile a menu item or palette row names: `SwitchProfile(id)`.
        A::SwitchProfile | A::OpenWindowInProfile | A::MoveWindowToProfile => return None,
        A::Help | A::Print | A::ExportText => return None,
        A::LineStart | A::LineEnd | A::DeleteLine | A::MoveItemUp | A::MoveItemDown | A::RowMenu => {
            return None;
        }
        // The Theme Studio's window reads its keys from the table (`keymap::studio_action`).
        A::SaveTheme | A::SaveThemeCopy => return None,
    })
}

/// The key bindings of every live action, as the table writes them (what a menu shows) and as
/// a US keyboard reports the shifted ones (⇧⌘] arrives as ⌘}). App keys have no context: they
/// work anywhere in a window, ahead of a view's own. The editing keys are GPUI Kit's text
/// fields' own and the terminal answers them itself, so they aren't bound here.
pub(crate) fn bindings() -> Vec<KeyBinding> {
    let terminal = KeyBindingContextPredicate::parse("Terminal").ok().map(Rc::new);
    // GPUI Kit binds ⌥⌘I to its inspector; it is the right panel's key now.
    let mut out = vec![KeyBinding::new("alt-cmd-i", Unbind("inspector::ToggleInspector".into()), None)];
    for entry in keymap::entries() {
        if entry.stage != Stage::Now || (entry.debug_only && !DEBUG) {
            continue;
        }
        let context = match entry.scope {
            Scope::App => None,
            Scope::Terminal => terminal.clone(),
            Scope::Edit | Scope::List | Scope::Studio => continue,
        };
        // The jump key is the user's to choose (`jump_waiting_key`): `bind_jump_key` binds it.
        if entry.id == ActionId::GoToWaiting {
            continue;
        }
        let Some(action) = gpui_action(entry.id) else { continue };
        for key in &entry.keys {
            for spelling in std::iter::once(key.clone()).chain(key.shifted_spelling()) {
                let text = spelling.to_string();
                let binding = KeyBinding::load(
                    &text,
                    action.boxed_clone(),
                    context.clone(),
                    false,
                    None,
                    &DummyKeyboardMapper,
                );
                match binding {
                    Ok(binding) => out.push(binding),
                    Err(err) => tracing::error!("binding {text} to {:?}: {err}", entry.id),
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use wormhole_core::keys::keymap::{ActionId, Scope, Stage, entries};

    use super::{bindings, gpui_action};

    #[test]
    fn every_live_action_has_a_gpui_action() {
        for entry in entries().iter().filter(|e| e.stage == Stage::Now) {
            let viewed = matches!(
                entry.id,
                ActionId::LineStart
                    | ActionId::LineEnd
                    | ActionId::DeleteLine
                    | ActionId::MoveItemUp
                    | ActionId::MoveItemDown
                    | ActionId::RowMenu
                    | ActionId::SaveTheme
                    | ActionId::SaveThemeCopy
                    | ActionId::SwitchProfile
                    | ActionId::OpenWindowInProfile
                    | ActionId::MoveWindowToProfile
            );
            assert_eq!(gpui_action(entry.id).is_some(), !viewed, "{:?}", entry.id);
        }
    }

    #[test]
    fn every_key_binds() {
        // Each live app or terminal key once as written, a shifted punctuation key (⇧⌘]) once more
        // as a US keyboard reports it (⌘}), and the Unbind of GPUI Kit's ⌥⌘I.
        let keys = entries()
            .iter()
            // The jump key comes from the settings (`bind_jump_key`).
            .filter(|e| e.stage == Stage::Now && gpui_action(e.id).is_some() && e.id != ActionId::GoToWaiting)
            .filter(|e| matches!(e.scope, Scope::App | Scope::Terminal))
            .flat_map(|e| &e.keys)
            .map(|k| 1 + usize::from(k.shifted_spelling().is_some()))
            .sum::<usize>();
        assert_eq!(bindings().len(), keys + 1);
    }

    /// The status item's menu writes its keys itself: they are the table's.
    #[test]
    fn the_status_item_shows_the_keys_the_app_binds() {
        use crate::menubar::MenuSnapshot;
        use crate::menubar::model::{Entry, KeyEquivalent, build};

        fn items(entries: &[Entry], out: &mut Vec<(String, Option<KeyEquivalent>)>) {
            for entry in entries {
                match entry {
                    Entry::Item(item) => out.push((item.title.clone(), item.key.clone())),
                    Entry::Submenu { entries, .. } => items(entries, out),
                    _ => {}
                }
            }
        }
        let mut found = Vec::new();
        items(&build(&MenuSnapshot::default(), std::time::Instant::now()).entries, &mut found);
        for (title, id) in [
            ("New Terminal", ActionId::NewTerminal),
            ("Open Folder…", ActionId::OpenFolder),
            ("Settings…", ActionId::Settings),
            ("Quit Wormhole", ActionId::Quit),
        ] {
            let shown = found.iter().find(|(t, _)| t == title).and_then(|(_, key)| key.clone());
            let bound = wormhole_core::keys::keymap::key(id).and_then(KeyEquivalent::from_spec);
            assert_eq!(shown, bound, "{title}");
        }
    }
}
