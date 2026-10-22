//! The app's actions as GPUI actions: their keys, bound from core's `keymap` table, and
//! who handles them. Actions about a window's terminals and panels are its `Workspace`'s (on
//! its root, so the focus must be inside it); the rest are the app's, whichever window is in
//! front. The menus (`app_menu`) and the tooltips read the same table, so a key is written
//! down once.
//!
//! A terminal takes keystrokes before GPUI's bindings (`TerminalKeys`), so a project action's
//! key is claimed there, first of all (projects keep priority, and `validate_key` keeps them
//! off the app's keys), and the app keys a terminal would send to its program (F6) are left to
//! the bindings.

use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::component::WindowExt as _;
use gpui_kit::component::input;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId, Scope, Stage};
use wormhole_core::keys::keyspec::KeySpec;
use wormhole_core::onboarding::intro::{Handover, Style};
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::sidebar::TerminalsIn;
use wormhole_core::view::sidebar::order::shortcut_target;

use crate::menubar::MenuCommand;
use crate::settings::{SettingsStore, settings};
use crate::terminal_view::{
    ClearBuffer, CopyCommand, CopyOutput, FONT_SIZES, NextPrompt, PreviousPrompt, RerunCommand, SelectOutput,
    TerminalKeys, keystroke_spec,
};
use crate::workspace::app::{self, AppState, app};
use crate::workspace::settings_page::update_settings;
use crate::workspace::{
    RightTab, Workspace, about, actions, ambient, app_menu, command_palette, focus, menu_bar, onboarding,
    settings_page, shortcut_sheet, sidebar, studio, updates,
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

/// Shell → New Terminal With: a new terminal running the shell with this key.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = workspace, no_json)]
pub struct NewTerminalWith(pub SharedString);

/// The inspector is in debug builds only (or with the `inspector` feature).
pub(super) const DEBUG: bool = cfg!(any(feature = "inspector", debug_assertions));

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
fn bindings() -> Vec<KeyBinding> {
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

/// Bind the keys, let project keys go first, and handle the actions that aren't one window's.
/// Before the menus are set: they read the bindings for their key equivalents.
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys(bindings());
    focus::init(cx);
    onboarding::init(cx);
    bind_jump_key(cx);
    cx.observe_global::<SettingsStore>(bind_jump_key).detach();
    TerminalKeys::claim_first(claim_window_key, cx);
    // Global listeners run inside the update of the window the action came from, which is then
    // unavailable: whatever opens or acts on windows is deferred.
    cx.on_action(|_: &NewWindow, cx| cx.defer(app::new_window_from_front));
    cx.on_action(|_: &CheckForUpdates, cx| {
        cx.defer(|cx| {
            settings_page::open_settings(Some(settings_page::Section::Updates), cx);
            updates::check_for_updates(true, cx);
        });
    });
    cx.on_action(|_: &NewProfile, cx| cx.defer(settings_page::open_new_profile_sheet));
    cx.on_action(|_: &ManageProfiles, cx| {
        cx.defer(|cx| settings_page::open_settings(Some(settings_page::Section::Profile), cx))
    });
    cx.on_action(|switch: &SwitchProfile, cx| {
        let id = switch.0.to_string();
        cx.defer(move |cx| {
            let Ok(id) = ProfileId::parse(&id) else { return };
            if let Err(message) = AppState::switch_profile(&id, cx) {
                tracing::warn!("switching profile: {message}");
            }
        });
    });
    cx.on_action(|open: &OpenWindowInProfile, cx| {
        let id = open.0.to_string();
        cx.defer(move |cx| {
            if let Ok(id) = ProfileId::parse(&id) {
                AppState::open_window_in_profile(&id, cx);
            }
        });
    });
    cx.on_action(|moved: &MoveWindowToProfile, cx| {
        let id = moved.0.to_string();
        cx.defer(move |cx| {
            let Ok(id) = ProfileId::parse(&id) else { return };
            if let Err(message) = AppState::move_front_window_to_profile(&id, cx) {
                tracing::warn!("moving the window: {message}");
            }
        });
    });
    cx.on_action(|_: &About, cx| cx.defer(about::open_about));
    cx.on_action(|_: &Licenses, cx| cx.defer(about::open_licenses));
    cx.on_action(|_: &OpenSettings, cx| cx.defer(|cx| settings_page::open_settings(None, cx)));
    // ⌘O: a terminal in the picked folder, in the window in front (the status item's too).
    cx.on_action(|_: &OpenFolder, cx| cx.defer(menu_bar::open_folder));
    // ⌥⌘J: the terminal that has waited longest, in any window.
    cx.on_action(|_: &GoToWaiting, cx| cx.defer(ambient::jump_to_waiting));
    // ⇧⌥⌘J: the terminal that finished last, then the one before it, in any window.
    cx.on_action(|_: &GoToFinished, cx| cx.defer(ambient::jump_to_finished));
    cx.on_action(|_: &ToggleThemeStudio, cx| cx.defer(theme_studio));
    // ⇧⌘P and ⌥⌘/: over the window in front, from any window and the terminal too.
    cx.on_action(|_: &ToggleCommandPalette, cx| cx.defer(command_palette::toggle));
    cx.on_action(|_: &ShowKeyboardShortcuts, cx| cx.defer(shortcut_sheet::toggle));
    cx.on_action(|_: &ToggleStatusBar, cx| update_settings(cx, |s| s.status_bar = !s.status_bar));
    cx.on_action(|_: &ToggleMinimap, cx| update_settings(cx, |s| s.minimap = !s.minimap));
    // Terminals in tabs or in the sidebar: every window at once.
    cx.on_action(|_: &ToggleTerminalTabs, cx| {
        update_settings(cx, |s| {
            s.terminals_in = match s.terminals_in {
                TerminalsIn::Sidebar => TerminalsIn::Tabs,
                TerminalsIn::Tabs => TerminalsIn::Sidebar,
            }
        })
    });
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &BringAllToFront, cx| crate::background::activate(cx));
    cx.on_action(|_: &Minimize, cx| in_front_window(cx, Window::minimize_window));
    cx.on_action(|_: &Zoom, cx| in_front_window(cx, Window::zoom_window));
    cx.on_action(|_: &ToggleFullScreen, cx| in_front_window(cx, Window::toggle_fullscreen));
    cx.on_action(|_: &ShowLogs, cx| cx.open_with_system(&crate::logging::dir()));
    cx.on_action(|_: &ShowIntro, cx| {
        cx.defer(|cx| {
            AppState::with_front_workspace(cx, |ws, window, cx| {
                crate::background::activate_window(window, cx);
                ws.show_intro(window, cx);
            });
        });
    });
    cx.on_action(|_: &PlayIntroEmerge, cx| play_intro_in_front(Style::Emerge, cx));
    cx.on_action(|_: &PlayIntroVortex, cx| play_intro_in_front(Style::Vortex, cx));
    cx.on_action(|_: &PlayIntroCollapse, cx| play_intro_in_front(Style::Collapse, cx));
    cx.on_action(|_: &PlayIntroFtl, cx| play_intro_in_front(Style::Ftl, cx));
    cx.on_action(|_: &PlayIntroAurora, cx| play_intro_in_front(Style::Aurora, cx));
    cx.on_action(|_: &PlayIntroTentacles, cx| play_intro_in_front(Style::Tentacles, cx));
    cx.on_action(|_: &PlayIntroVoyage, cx| play_intro_in_front(Style::Voyage, cx));
    cx.on_action(|_: &PlayWelcomeCursor, cx| {
        crate::workspace::welcome::replay_in_front(Handover::Cursor, cx)
    });
    cx.on_action(|_: &PlayWelcomePortal, cx| {
        crate::workspace::welcome::replay_in_front(Handover::Portal, cx)
    });
    cx.on_action(|_: &PlayWelcomeLogo, cx| crate::workspace::welcome::replay_in_front(Handover::Logo, cx));
    cx.on_action(|recent: &OpenRecent, cx| {
        let root = recent.0.clone();
        cx.defer(move |cx| menu_bar::run_command(MenuCommand::OpenProject(root), cx));
    });
    // A key that ran an action may have flipped a label or moved a check mark: bring the menus
    // up to date now rather than on the next tick (typing runs no action, so it costs nothing).
    cx.observe_keystrokes(|event, _, cx| {
        if event.action.is_some() {
            cx.defer(app_menu::sync);
        }
    })
    .detach();
}

/// Wormhole → Theme Studio (⌥⌘,): the Studio to the front, opened if need be. With the Studio
/// already in front it closes it, as its key always did.
fn theme_studio(cx: &mut App) {
    let in_front = app(cx)
        .studio_window
        .as_ref()
        .zip(cx.active_window())
        .is_some_and(|((studio, _), active)| studio.window_id() == active.window_id());
    if in_front { studio::toggle_studio(cx) } else { studio::open_studio(cx) }
}

/// The jump-to-waiting key now bound, as `jump_waiting_key` said it.
struct JumpKey(String);

impl Global for JumpKey {}

/// Bind `jump_waiting_key` (⌥⌘J unless the user recorded another) to Go to Waiting Terminal, and
/// again when the setting changes: the old key is unbound, so it goes back to whatever else has
/// it, and the menus are put up again to show the new one.
fn bind_jump_key(cx: &mut App) {
    let spec = settings(cx).jump_waiting_key.trim().to_string();
    let bound = cx.try_global::<JumpKey>().map(|key| key.0.clone());
    if bound.as_deref() == Some(spec.as_str()) {
        return;
    }
    let spellings = |text: &str| -> Vec<String> {
        let Ok(key) = text.parse::<KeySpec>() else { return Vec::new() };
        std::iter::once(key.clone()).chain(key.shifted_spelling()).map(|k| k.to_string()).collect()
    };
    let mut bindings = Vec::new();
    for text in bound.as_deref().map(spellings).unwrap_or_default() {
        let unbind = Box::new(Unbind(GoToWaiting.name().into()));
        bindings.extend(KeyBinding::load(&text, unbind, None, false, None, &DummyKeyboardMapper).ok());
    }
    for text in spellings(&spec) {
        match KeyBinding::load(&text, Box::new(GoToWaiting), None, false, None, &DummyKeyboardMapper) {
            Ok(binding) => bindings.push(binding),
            Err(err) => tracing::warn!("jump_waiting_key {text}: {err}"),
        }
    }
    cx.bind_keys(bindings);
    cx.set_global(JumpKey(spec));
    app_menu::rebuild(cx);
}

/// Play the intro `style` around the window in front (About's buttons, the keys): brought forward
/// first, as About may be in front of it. Here, not in `intro`, which only knows what it draws.
fn play_intro_in_front(style: Style, cx: &mut App) {
    cx.defer(move |cx| {
        AppState::with_front_workspace(cx, |_, window, cx| {
            crate::background::activate_window(window, cx);
            crate::intro::play(window, style, cx);
        });
    });
}

/// Run `f` on the window in front (any of ours, not only a terminal window), once the update
/// that dispatched the action is over.
fn in_front_window(cx: &mut App, f: fn(&Window)) {
    if let Some(handle) = cx.active_window() {
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, _| f(window));
        });
    }
}

/// Keys the window takes before the terminal or a binding sees them (but in a dialog): Esc while
/// a diff covers the terminal that has the keys, and a project action's key, wherever the focus is.
fn claim_window_key(keystroke: &Keystroke, window: &mut Window, cx: &mut App) -> bool {
    let escape = keystroke.key == "escape" && !keystroke.modifiers.modified();
    // A project key carries ⌘ or is an F-key (`validate_key`): typing never gets further.
    let m = &keystroke.modifiers;
    if !escape && !m.platform && !keystroke_spec(keystroke).is_function_key() {
        return false;
    }
    let id = window.window_handle().window_id();
    let workspace = cx
        .try_global::<AppState>()
        .and_then(|state| state.windows.iter().find(|w| w.handle.window_id() == id))
        .and_then(|w| w.workspace.upgrade());
    let Some(workspace) = workspace else { return false };
    if window.has_active_dialog(cx) {
        return false;
    }
    if escape {
        return workspace.update(cx, |ws, cx| ws.terminal_escape(window, cx));
    }
    let Some(action) = workspace.read(cx).project_action_for_key(keystroke, cx) else { return false };
    workspace.update(cx, |ws, cx| ws.run_project_action(&action, window, cx));
    true
}

/// The zoom (points over the Settings size) after one more step, kept within [`FONT_SIZES`].
fn zoomed(base: f32, zoom: f32, step: f32) -> f32 {
    (base + zoom + step).clamp(*FONT_SIZES.start(), *FONT_SIZES.end()) - base
}

impl Workspace {
    /// The window's own actions, on its root element. An action that can't do anything for the
    /// terminal in front isn't handled at all, which is what greys its menu item out.
    pub(super) fn with_actions(&self, root: Div, cx: &mut Context<Self>) -> Div {
        use wormhole_core::view::panes::{Axis, Direction, Preset};
        let has_folder = self.active_terminal().read(cx).cwd.is_some();
        // A worktree needs a repository: the terminal's folder has a git status.
        let has_repo = self.active_session().git.status.is_some();
        let split = self.active_split().is_some();
        root.on_action(cx.listener(|this, _: &NewSession, window, cx| this.new_session(window, cx)))
            .on_action(cx.listener(|this, _: &NewTerminalNoGroup, window, cx| {
                let cwd = this.default_cwd(cx);
                this.open_session_in(Some(None), None, cwd, window, cx);
            }))
            .when(has_repo, |root| {
                root.on_action(cx.listener(|this, _: &NewTerminalInWorktree, window, cx| {
                    this.new_terminal_in_worktree(window, cx)
                }))
            })
            // Panes: a split or a layout only with the room for it, the rest only
            // in a split, so the menu greys them out otherwise.
            .when(self.can_split(Axis::Row), |root| {
                root.on_action(
                    cx.listener(|this, _: &SplitRight, window, cx| this.split_pane(Axis::Row, window, cx)),
                )
            })
            .when(self.can_split(Axis::Column), |root| {
                root.on_action(
                    cx.listener(|this, _: &SplitDown, window, cx| this.split_pane(Axis::Column, window, cx)),
                )
            })
            .when(split, |root| {
                root.on_action(cx.listener(|this, _: &ZoomPane, _, cx| this.toggle_zoom(cx)))
                    .on_action(cx.listener(|this, _: &LeaveSplit, _, cx| {
                        this.leave_split(this.active, cx);
                    }))
                    .on_action(cx.listener(|this, _: &FocusPaneLeft, window, cx| {
                        this.focus_pane_toward(Direction::Left, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &FocusPaneRight, window, cx| {
                        this.focus_pane_toward(Direction::Right, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &FocusPaneUp, window, cx| {
                        this.focus_pane_toward(Direction::Up, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &FocusPaneDown, window, cx| {
                        this.focus_pane_toward(Direction::Down, window, cx)
                    }))
            })
            .when(self.can_layout(Preset::TwoColumns), |root| {
                root.on_action(cx.listener(|this, _: &LayoutColumns, window, cx| {
                    this.apply_layout(Preset::TwoColumns, window, cx)
                }))
            })
            .when(self.can_layout(Preset::TwoRows), |root| {
                root.on_action(cx.listener(|this, _: &LayoutRows, window, cx| {
                    this.apply_layout(Preset::TwoRows, window, cx)
                }))
            })
            .when(self.can_layout(Preset::Grid), |root| {
                root.on_action(
                    cx.listener(|this, _: &LayoutGrid, window, cx| {
                        this.apply_layout(Preset::Grid, window, cx)
                    }),
                )
            })
            .on_action(
                cx.listener(|this, _: &LayoutSingle, window, cx| {
                    this.apply_layout(Preset::Single, window, cx)
                }),
            )
            .on_action(cx.listener(|this, with: &NewTerminalWith, window, cx| {
                this.new_session_with(Some(with.0.to_string()), window, cx)
            }))
            .on_action(cx.listener(|this, _: &CloseTerminal, window, cx| this.close_selected(window, cx)))
            .on_action(cx.listener(|this, _: &CloseWindow, window, cx| this.close_window(window, cx)))
            .on_action(cx.listener(|this, _: &NextTerminal, window, cx| this.step_session(1, window, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousTerminal, window, cx| this.step_session(-1, window, cx)),
            )
            .on_action(cx.listener(|this, n: &GoToTerminal, window, cx| this.go_to_numbered(n.0, window, cx)))
            .on_action(cx.listener(|this, _: &NextGroup, window, cx| this.step_group(true, window, cx)))
            .on_action(cx.listener(|this, _: &PreviousGroup, window, cx| this.step_group(false, window, cx)))
            .on_action(cx.listener(|this, n: &GoToGroup, window, cx| this.go_to_group(n.0, window, cx)))
            .on_action(cx.listener(|this, _: &PinTerminal, _, cx| this.toggle_pin(this.active, cx)))
            .when(has_folder, |root| {
                root.on_action(cx.listener(|this, _: &RevealInFinder, _, cx| {
                    if let Some(cwd) = this.active_terminal().read(cx).cwd.clone() {
                        sidebar::reveal_in_finder(&cwd, cx);
                    }
                }))
                .on_action(cx.listener(|this, _: &CopyPath, _, cx| {
                    if let Some(cwd) = this.active_terminal().read(cx).cwd.clone() {
                        cx.write_to_clipboard(ClipboardItem::new_string(cwd.to_string_lossy().into_owned()));
                    }
                }))
            })
            .on_action(cx.listener(|this, action: &RunProjectAction, window, cx| {
                this.run_project_action(&action.0, window, cx)
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(cx.listener(|this, _: &ToggleRightPanel, window, cx| this.toggle_notes(window, cx)))
            .on_action(cx.listener(|this, _: &ShowChanges, _, cx| this.show_tab(RightTab::Changes, cx)))
            .on_action(cx.listener(|this, _: &ShowHistory, _, cx| this.show_tab(RightTab::History, cx)))
            .when(self.usage_tab_on(cx), |root| {
                root.on_action(cx.listener(|this, _: &ShowUsage, _, cx| this.show_tab(RightTab::Usage, cx)))
            })
            // Any terminal whose folder is known has the tab, with a project or not.
            .when(has_folder, |root| {
                root.on_action(
                    cx.listener(|this, _: &ShowProject, _, cx| this.show_tab(RightTab::Project, cx)),
                )
            })
            .on_action(
                cx.listener(|this, _: &FocusTerminalOrPanel, window, cx| this.toggle_focus(window, cx)),
            )
            .on_action(cx.listener(|this, _: &FocusNextArea, window, cx| this.step_focus_area(1, window, cx)))
            .on_action(
                cx.listener(|this, _: &FocusPreviousArea, window, cx| this.step_focus_area(-1, window, cx)),
            )
            .on_action(cx.listener(|this, _: &Bigger, _, cx| this.zoom_font(Some(1.0), cx)))
            .on_action(cx.listener(|this, _: &Smaller, _, cx| this.zoom_font(Some(-1.0), cx)))
            .on_action(cx.listener(|this, _: &ActualSize, _, cx| this.zoom_font(None, cx)))
            .when(crate::workspace::notes::notes_on(cx), |root| {
                root.on_action(cx.listener(|this, _: &ShowNotes, _, cx| this.show_tab(RightTab::Notes, cx)))
                    .on_action(cx.listener(|this, _: &NewNote, window, cx| this.new_note(None, window, cx)))
                    .on_action(cx.listener(|this, _: &StashPrompt, window, cx| this.stash_prompt(window, cx)))
            })
            // Find works on the terminal in front, so not while a diff covers it.
            .when(self.diff_view.is_none(), |root| {
                root.on_action(cx.listener(|this, _: &Find, window, cx| {
                    let terminal = this.active_terminal().clone();
                    terminal.update(cx, |terminal, cx| {
                        let seed = terminal.find_seed();
                        terminal.open_find(seed, window, cx);
                    });
                }))
                .on_action(cx.listener(|this, _: &FindNext, window, cx| this.find_step(1, window, cx)))
                .on_action(cx.listener(|this, _: &FindPrevious, window, cx| this.find_step(-1, window, cx)))
                .on_action(cx.listener(|this, _: &UseSelectionForFind, window, cx| {
                    let terminal = this.active_terminal().clone();
                    terminal.update(cx, |terminal, cx| terminal.use_selection_for_find(window, cx));
                }))
            })
    }

    /// ⌘G / ⇧⌘G: the front terminal's next or previous match.
    fn find_step(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let terminal = self.active_terminal().clone();
        terminal.update(cx, |terminal, cx| terminal.find_step(delta, window, cx));
    }

    /// ⌘1–9: the terminal whose keycap says ⌘n; past the last numbered one, that one.
    fn go_to_numbered(&mut self, n: u8, window: &mut Window, cx: &mut Context<Self>) {
        let numbered = self.session_listing(cx).numbered();
        if let Some(ix) = shortcut_target(&numbered, usize::from(n)) {
            self.activate(ix, window, cx);
        }
    }

    /// ⌘+ / ⌘- (`Some(±1)`) and ⌘0 (`None`): this window's terminal font a point bigger or
    /// smaller, or back to the size in Settings. A view setting, as in Terminal.app: not saved.
    pub(super) fn zoom_font(&mut self, step: Option<f32>, cx: &mut Context<Self>) {
        let zoom = step.map_or(0.0, |step| zoomed(settings(cx).font_size, self.font_zoom, step));
        if zoom == self.font_zoom {
            return;
        }
        self.font_zoom = zoom;
        for pane in self.sessions.iter().map(|session| &session.pane) {
            pane.terminal.update(cx, |terminal, cx| terminal.set_font_zoom(zoom, cx));
        }
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::keys::keymap::{ActionId, Scope, Stage, entries};

    use super::{bindings, gpui_action, zoomed};

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

    #[test]
    fn font_zoom_stays_within_what_settings_allow() {
        assert_eq!(zoomed(14.0, 0.0, 1.0), 1.0);
        assert_eq!(zoomed(14.0, 3.0, -1.0), 2.0);
        assert_eq!(zoomed(35.0, 1.0, 1.0), 1.0, "36 at most");
        assert_eq!(zoomed(9.0, -1.0, -1.0), -1.0, "8 at least");
    }
}
