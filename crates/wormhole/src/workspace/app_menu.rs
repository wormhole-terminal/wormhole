//! The menu bar: core's `keymap::menus` filled in for the window in front and handed to
//! AppKit, built again whenever what it shows changes (a label that flips, the checked layout,
//! the terminals ⌘1–9, recent folders, the project's actions). The key equivalents come from
//! the bindings (`actions::init`), so menus show them natively; AppKit asks before showing an
//! item whether anything in front handles it, and greys it out if not.
//!
//! GPUI keeps the actions of every menu bar it was given for the life of the app, so the menus
//! are not rebuilt for each step of a title an agent animates: a change of names alone waits
//! [`NAMES_EVERY`].
use wormhole_core::keymap::ActionId;
use wormhole_core::keymap::menus::{self, Command, Item, MenuState};
use wormhole_core::project::Feature;
use wormhole_core::shells;

use super::actions::{
    DEBUG, MoveWindowToProfile, NewTerminalWith, OpenRecent, OpenWindowInProfile, Placeholder,
    RunProjectAction, SwitchProfile, gpui_action,
};
use super::app::{AppState, app};
use super::*;

/// The longest the Window menu's terminal names may lag behind the sidebar's.
const NAMES_EVERY: Duration = Duration::from_secs(5);

/// What the menus were last built for, and when.
struct Built {
    state: MenuState,
    at: Instant,
}

impl Global for Built {}

/// Put the menus up for the window in front, if they changed. Runs on every app tick and after
/// a key ran an action; when nothing changed it only compares.
pub(crate) fn sync(cx: &mut App) {
    let state = state(cx);
    if let Some(built) = cx.try_global::<Built>() {
        let names_only = built.state.differs_only_in_names(&state);
        if built.state == state || (names_only && built.at.elapsed() < NAMES_EVERY) {
            return;
        }
    }
    let recent = app(cx).menu.recent().to_vec();
    let shells: Vec<String> = shells::choices(settings(cx)).into_iter().map(|choice| choice.key).collect();
    let profiles: Vec<SharedString> =
        app(cx).profiles.profiles().iter().map(|p| p.id.to_string().into()).collect();
    cx.set_menus(gpui_menus(menus::menus(&state), &recent, &shells, &profiles));
    cx.set_global(Built { state, at: Instant::now() });
}

/// Put the menus up again on the next sync, whatever they show (a key binding changed).
pub(crate) fn rebuild(cx: &mut App) {
    if cx.has_global::<Built>() {
        cx.remove_global::<Built>();
    }
}

/// The menus as last put up, as text (`devctl menus` off macOS).
#[cfg(feature = "devctl")]
pub(crate) fn dump(cx: &App) -> String {
    cx.try_global::<Built>().map(|built| menus::dump(&menus::menus(&built.state))).unwrap_or_default()
}

/// What the menus show for the window in front (the command palette offers the same actions).
pub(super) fn state(cx: &mut App) -> MenuState {
    let front = AppState::front_window(cx);
    let full_screen = front
        .as_ref()
        .and_then(|front| front.handle.update(cx, |_, window, _| window.is_fullscreen()).ok())
        .unwrap_or(false);
    let mut state = front
        .and_then(|front| front.workspace.read_with(cx, |ws, cx| ws.menu_state(cx)).ok())
        .unwrap_or_default();
    let settings = settings(cx);
    if !settings.status_bar {
        state.off.push(ActionId::ToggleStatusBar);
    }
    if !settings.minimap {
        state.off.push(ActionId::ToggleMinimap);
    }
    if settings.terminals_in == wormhole_core::sidebar::TerminalsIn::Sidebar {
        state.off.push(ActionId::ToggleTerminalTabs);
    }
    if !full_screen {
        state.off.push(ActionId::FullScreen);
    }
    state.recent = app(cx).menu.recent().iter().map(|root| folder_name(root)).collect();
    state.shells = shells::choices(settings).into_iter().map(|choice| choice.name).collect();
    let list = &app(cx).profiles;
    state.profiles = list.profiles().iter().map(|p| p.name.clone()).collect();
    state.current_profile = list.profiles().iter().position(|p| &p.id == list.active_id());
    state.window_profile = AppState::front_window(cx)
        .and_then(|front| list.profiles().iter().position(|p| p.id == front.profile));
    state.debug = DEBUG;
    state
}

fn folder_name(root: &Path) -> String {
    root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| root.display().to_string())
}

impl Workspace {
    /// What the menus show about this window.
    fn menu_state(&self, cx: &App) -> MenuState {
        use ActionId::*;
        let mut state = MenuState::default();
        if !self.sidebar_shown() {
            state.off.push(ToggleSidebar);
        }
        if !self.notes_visible() {
            state.off.push(ToggleRightPanel);
        }
        state.checked.push(match self.right_tab {
            RightTab::Notes => ShowNotes,
            RightTab::Changes => ShowChanges,
            RightTab::History => ShowHistory,
            RightTab::Project => ShowProject,
            RightTab::Usage => ShowUsage,
        });
        if self.active_session().pinned {
            state.checked.push(PinTerminal);
        }
        self.pane_menu_state(&mut state);
        if self.active_terminal().read(cx).cwd.is_none() {
            state.disabled.extend([RevealInFinder, CopyPath]);
        }
        // The Project tab needs a folder, and the whole project-folder feature behind its flag.
        if self.active_folder(cx).is_none() || !super::project::project_folders_on(cx) {
            state.disabled.push(ShowProject);
        }
        if let Some(project) =
            self.active_project(cx).filter(|project| app(cx).hub.enabled(project, Feature::Actions))
        {
            state.project_actions =
                project.actions().iter().map(|a| (a.id.clone(), a.title.clone())).collect();
        }
        let numbered = self.session_listing(cx).numbered();
        state.terminals = numbered.iter().map(|&ix| self.sessions[ix].name(cx).to_string()).collect();
        state.current_terminal = numbered.iter().position(|&ix| ix == self.active);
        state
    }
}

/// GPUI's menus for the model. `recent` is the list the Open Recent items were named from, and
/// `shells` the keys of the shells the New With items were named from, so each item opens what
/// it showed even if the list changes before the click.
fn gpui_menus(
    menus: Vec<menus::Menu>,
    recent: &[PathBuf],
    shells: &[String],
    profiles: &[SharedString],
) -> Vec<Menu> {
    menus
        .into_iter()
        .map(|menu| Menu {
            name: menu.title.into(),
            items: gpui_items(menu.items, recent, shells, profiles),
            disabled: false,
        })
        .collect()
}

fn gpui_items(
    items: Vec<Item>,
    recent: &[PathBuf],
    shells: &[String],
    profiles: &[SharedString],
) -> Vec<MenuItem> {
    items
        .into_iter()
        .filter_map(|item| {
            Some(match item {
                Item::Separator => MenuItem::separator(),
                Item::Services => MenuItem::os_submenu("Services", SystemMenuType::Services),
                Item::Submenu { title, items, enabled } => MenuItem::Submenu(Menu {
                    name: title.into(),
                    items: gpui_items(items, recent, shells, profiles),
                    disabled: !enabled,
                }),
                // Greyed like a disabled action; its action has no handler, so AppKit's own
                // validation leaves it so too.
                Item::Placeholder(title) => MenuItem::Action {
                    name: title.into(),
                    action: Box::new(Placeholder),
                    os_action: None,
                    checked: false,
                    disabled: true,
                },
                Item::Action { command, title, checked, enabled, .. } => {
                    let (action, os_action): (Box<dyn Action>, _) = match command {
                        Command::Action(id) => (gpui_action(id)?, os_action(id)),
                        Command::OpenRecent(ix) => (Box::new(OpenRecent(recent.get(ix)?.clone())), None),
                        Command::RunProjectAction(id) => (Box::new(RunProjectAction(id.into())), None),
                        Command::NewTerminalWith(ix) => {
                            (Box::new(NewTerminalWith(shells.get(ix)?.clone().into())), None)
                        }
                        Command::SwitchProfile(ix) => {
                            (Box::new(SwitchProfile(profiles.get(ix)?.clone())), None)
                        }
                        Command::OpenInProfile(ix) => {
                            (Box::new(OpenWindowInProfile(profiles.get(ix)?.clone())), None)
                        }
                        Command::MoveToProfile(ix) => {
                            (Box::new(MoveWindowToProfile(profiles.get(ix)?.clone())), None)
                        }
                    };
                    MenuItem::Action { name: title.into(), action, os_action, checked, disabled: !enabled }
                }
            })
        })
        .collect()
}

/// The editing items are the system's own (`cut:`, `copy:` …), so AppKit treats them as such.
fn os_action(id: ActionId) -> Option<OsAction> {
    Some(match id {
        ActionId::Undo => OsAction::Undo,
        ActionId::Redo => OsAction::Redo,
        ActionId::Cut => OsAction::Cut,
        ActionId::Copy => OsAction::Copy,
        ActionId::Paste => OsAction::Paste,
        ActionId::SelectAll => OsAction::SelectAll,
        _ => return None,
    })
}
