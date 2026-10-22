//! The status item, the Dock (badge and menu) and the global hotkey: keeps them in step with the
//! settings and the terminals of every window, and carries out what the user picks.
//!
//! Above the window: it may use the window and the shared state; the app root uses it, and so does
//! the shared state (`AppState`), which holds the status item's [`MenuState`] as a field and builds
//! it. That is the one edge back down, kept until `MenuState` is a global of its own.

pub(crate) mod quick_panel;

use std::path::Path;
#[cfg(feature = "devctl")]
use std::path::PathBuf;
use std::time::Instant;

use gpui_kit::*;
use wormhole_core::config::HotkeyAction;
use wormhole_core::keys::keyspec::KeySpec;
use wormhole_core::notify::{self, Counts, Terminal};
use wormhole_core::project::Feature;

use crate::app_state::{AppState, app, app_mut};
use crate::menubar::model::{self, DockEntry, KeyEquivalent};
use crate::menubar::{ActionItem, MenuCommand, MenuSnapshot, StatusColors};
use crate::platform::hotkey::Hotkey;
use crate::platform::status_item::{self, MenuBar};
use crate::settings::{settings, ui};
use crate::settings_window;
use crate::updates;
use crate::workspace::window_lifecycle::open_tab;
use crate::workspace::window_lifecycle::with_front_workspace;

/// A terminal in the Dock's menu.
#[derive(Clone, PartialEq, Debug, gpui_kit::Action)]
#[action(namespace = dock, no_json)]
pub(crate) struct DockOpenTab {
    pub tab: i64,
}

actions!(dock, [DockNewTerminal, DockNewWindow, DockHeader]);

pub(crate) struct MenuState {
    tx: smol::channel::Sender<MenuCommand>,
    bar: Option<MenuBar>,
    hotkey: Option<Hotkey>,
    /// The hotkey setting last applied, so a bad spec isn't retried every tick.
    applied_hotkey: Option<String>,
    /// Held off while the settings window records a new one: the keys pressed then reach
    /// the recorder, the current hotkey's included.
    hotkey_paused: bool,
    pub hotkey_error: Option<String>,
    /// The working animation while it runs (dropping it stops it).
    animation: Option<Task<()>>,
    /// The Dock menu as last given to GPUI, which keeps every action it is ever handed: so it
    /// is only replaced when its words change.
    dock_menu: Option<Vec<DockEntry>>,
    /// The Dock menu's actions have their handlers.
    dock_actions: bool,
}

impl MenuState {
    /// Menu clicks and hotkey presses arrive on the returned receiver.
    pub fn new() -> (Self, smol::channel::Receiver<MenuCommand>) {
        let (tx, rx) = smol::channel::unbounded();
        let state = Self {
            tx,
            bar: None,
            hotkey: None,
            applied_hotkey: None,
            hotkey_paused: false,
            hotkey_error: None,
            animation: None,
            dock_menu: None,
            dock_actions: false,
        };
        (state, rx)
    }

    /// A profile switch: the hotkey is registered again as the new profile sets it. The status
    /// item stays: whether it shows, and its icon, follow the settings every tick.
    pub fn switch_profile(&mut self) {
        self.applied_hotkey = None;
    }

    /// Hold the hotkey off while its field in Settings records, or put it back.
    pub fn pause_hotkey(&mut self, paused: bool) {
        if self.hotkey_paused == paused {
            return;
        }
        self.hotkey_paused = paused;
        if let Some(spec) = self.applied_hotkey.clone() {
            self.apply_hotkey(&spec);
        }
    }

    fn apply_hotkey(&mut self, spec: &str) {
        // Release the old key first, or re-registering the same combination fails.
        self.hotkey = None;
        self.hotkey_error = None;
        self.applied_hotkey = Some(spec.to_string());
        if !registers(spec, self.hotkey_paused) {
            return;
        }
        match Hotkey::register(spec) {
            Ok(key) => {
                let tx = self.tx.clone();
                Hotkey::on_press(move || {
                    let _ = tx.try_send(MenuCommand::Hotkey);
                });
                self.hotkey = Some(key);
            }
            Err(err) => {
                tracing::warn!("global hotkey: {err}");
                self.hotkey_error = Some(err);
            }
        }
    }
}

/// Whether a hotkey spec is to be registered now: never an empty one, and not while the
/// recorder in Settings has the keys.
fn registers(spec: &str, paused: bool) -> bool {
    !spec.is_empty() && !paused
}

/// Called every tick: cheap, since the menu is only built as it opens and the icon, the badge
/// and the Dock menu are only touched when they change.
pub(crate) fn sync(cx: &mut App) {
    let (enabled, spec) = {
        let s = settings(cx);
        (s.menu_bar, s.hotkey.trim().to_string())
    };
    let mut hotkey_changed = false;
    {
        let menu = &mut app_mut(cx).menu;
        if enabled && menu.bar.is_none() {
            menu.bar = status_item::install(menu.tx.clone());
        } else if !enabled {
            menu.bar = None;
            menu.animation = None;
        }
        if menu.applied_hotkey.as_deref() != Some(spec.as_str()) {
            menu.apply_hotkey(&spec);
            hotkey_changed = true;
        }
    }
    if hotkey_changed {
        // The settings window shows the hotkey's error.
        cx.refresh_windows();
    }
    register_dock_actions(cx);
    let terminals = crate::workspace::ambient::terminals(cx);
    sync_dock(&terminals, cx);
    if app(cx).menu.bar.is_none() {
        return;
    }
    let snapshot = snapshot(terminals, cx);
    let animate = model::status_icon(&snapshot.terminals, snapshot.icon, snapshot.animate).animate;
    if let Some(bar) = &mut app_mut(cx).menu.bar {
        bar.update(&snapshot);
    }
    animate_icon(animate, cx);
}

fn rgba(color: Hsla) -> crate::menubar::Rgba {
    let rgba = gpui_kit::Rgba::from(color);
    [rgba.r, rgba.g, rgba.b, rgba.a]
}

/// The actions of the projects that have a terminal open, in terminal order (the menu's
/// Projects submenu, the quick panel's Do section).
pub(crate) fn project_actions(cx: &App) -> Vec<ActionItem> {
    let state = app(cx);
    let mut roots = Vec::new();
    for window in state.windows.iter().filter(|w| !w.hidden) {
        if let Ok(window_roots) = window.workspace.read_with(cx, |ws, cx| ws.project_roots(cx)) {
            roots.extend(window_roots);
        }
    }

    let mut actions = Vec::new();
    let mut seen = Vec::new();
    for root in &roots {
        if seen.contains(&root) {
            continue;
        }
        seen.push(root);
        let Some(project) = state.hub.projects.get(root) else { continue };
        if !state.hub.enabled(project, Feature::Actions) {
            continue;
        }
        actions.extend(project.actions().iter().map(|action| ActionItem {
            project: project_name(root),
            root: root.clone(),
            id: action.id.clone(),
            title: action.title.clone(),
        }));
    }
    actions
}

fn snapshot(terminals: Vec<Terminal>, cx: &App) -> MenuSnapshot {
    let actions = project_actions(cx);
    let state = app(cx);
    let menu = &state.menu;
    let recent_projects =
        state.recent_projects.roots().iter().map(|root| (project_name(root), root.clone())).collect();
    // A hotkey held off while a new one is recorded shows as it would be (as background mode's,
    // which holds nothing, does).
    let registered = menu.hotkey.is_some() || (menu.hotkey_paused && menu.hotkey_error.is_none());
    let hotkey = menu.applied_hotkey.as_deref().filter(|_| registered);
    let key = |spec: &str| spec.parse::<KeySpec>().ok().and_then(|spec| KeyEquivalent::from_spec(&spec));
    let s = settings(cx);
    let ui = ui(cx);
    MenuSnapshot {
        terminals,
        actions,
        recent_projects,
        hotkey: hotkey.and_then(key),
        jump_key: key(&s.jump_waiting_key),
        icon: s.menu_bar_icon,
        // Reduce Motion stops it.
        animate: s.menu_bar_animate && !cx.reduce_motion(),
        colors: StatusColors {
            attention: rgba(ui.agent_attention),
            done: rgba(ui.agent_done),
            working: rgba(ui.agent_working),
            danger: rgba(ui.danger),
            running: rgba(ui.muted),
        },
    }
}

/// Start or stop the working animation: [`notify::MENU_ANIM_FRAMES`] frames,
/// [`notify::MENU_ANIM_FRAME`] each, only while working is what the icon shows.
fn animate_icon(animate: bool, cx: &mut App) {
    let running = app(cx).menu.animation.is_some();
    if animate == running {
        return;
    }
    if !animate {
        let menu = &mut app_mut(cx).menu;
        menu.animation = None;
        if let Some(bar) = &mut menu.bar {
            bar.show_frame(None);
        }
        return;
    }
    let task = cx.spawn(async move |cx| {
        let mut frame = 0;
        loop {
            cx.background_executor().timer(notify::MENU_ANIM_FRAME).await;
            frame = (frame + 1) % notify::MENU_ANIM_FRAMES;
            cx.update(|cx| {
                if let Some(bar) = &mut app_mut(cx).menu.bar {
                    bar.show_frame(Some(frame));
                }
            });
        }
    });
    app_mut(cx).menu.animation = Some(task);
}

/// The Dock's badge (`dock_badge`) and its menu.
fn sync_dock(terminals: &[Terminal], cx: &mut App) {
    let counts = Counts::of(terminals);
    let badge = notify::dock_badge(settings(cx).dock_badge, counts.needs_you, counts.results());
    crate::platform::dock::set_badge(badge, cx);
    let menu = model::dock_menu(terminals, Instant::now());
    if app(cx).menu.dock_menu.as_ref() == Some(&menu) {
        return;
    }
    let items = menu
        .iter()
        .map(|entry| match entry {
            DockEntry::Header(title) => MenuItem::Action {
                name: (*title).into(),
                action: Box::new(DockHeader),
                os_action: None,
                checked: false,
                disabled: true,
            },
            DockEntry::Terminal { label, tab } => MenuItem::action(label.clone(), DockOpenTab { tab: *tab }),
            DockEntry::Separator => MenuItem::separator(),
            DockEntry::NewTerminal => MenuItem::action("New Terminal", DockNewTerminal),
            DockEntry::NewWindow => MenuItem::action("New Window", DockNewWindow),
        })
        .collect();
    cx.set_dock_menu(items);
    app_mut(cx).menu.dock_menu = Some(menu);
}

fn register_dock_actions(cx: &mut App) {
    if std::mem::replace(&mut app_mut(cx).menu.dock_actions, true) {
        return;
    }
    // A menu's action runs inside the update of the window that was active: act afterwards.
    cx.on_action(|action: &DockOpenTab, cx| {
        let tab = action.tab;
        cx.defer(move |cx| run_command(MenuCommand::OpenTab(tab), cx));
    });
    cx.on_action(|_: &DockNewTerminal, cx| cx.defer(|cx| run_command(MenuCommand::NewSession, cx)));
    cx.on_action(|_: &DockNewWindow, cx| {
        cx.defer(|cx| {
            crate::background::activate(cx);
            if with_front_workspace(cx, |ws, window, cx| ws.new_window(window, cx)).is_none()
                && app(cx).windows.is_empty()
            {
                crate::workspace::window_lifecycle::open_workspace_window(None, None, cx);
            }
        })
    });
    cx.on_action(|_: &DockHeader, _| {});
}

/// Carry out a menu bar / hotkey command, in whichever window it belongs to.
pub(crate) fn run_command(command: MenuCommand, cx: &mut App) {
    if command == MenuCommand::Hotkey {
        match settings(cx).hotkey_action {
            // Over whatever is in front, without bringing Wormhole forward.
            HotkeyAction::QuickPanel => {
                crate::status_item::quick_panel::toggle(cx);
                return;
            }
            HotkeyAction::ToggleApp if cx.active_window().is_some() => {
                cx.hide();
                return;
            }
            HotkeyAction::ToggleApp => {}
        }
    }
    if command == MenuCommand::Quit {
        cx.defer(crate::workspace::windows::request_quit);
        return;
    }
    crate::background::activate(cx);
    if let Some(front) = AppState::front_window(cx) {
        let _ = front.handle.update(cx, |_, window, cx| crate::background::activate_window(window, cx));
    }
    match command {
        MenuCommand::ShowApp | MenuCommand::Hotkey | MenuCommand::Quit => {}
        MenuCommand::NewSession => {
            with_front_workspace(cx, |ws, window, cx| ws.new_session(window, cx));
        }
        MenuCommand::OpenFolder => open_folder(cx),
        MenuCommand::OpenTab(tab) => open_tab(tab, cx),
        MenuCommand::JumpToWaiting => crate::workspace::ambient::jump_to_waiting(cx),
        MenuCommand::RunAction { root, id } => {
            // Actions run in the context of their project's terminal.
            let owner = AppState::find_window(cx, |ws, _| ws.session_for_project(&root).is_some());
            if let Some(owner) = owner {
                AppState::with_workspace(&owner, cx, |ws, window, cx| {
                    crate::background::activate_window(window, cx);
                    if let Some(ix) = ws.session_for_project(&root) {
                        ws.activate(ix, window, cx);
                        ws.run_project_action(&id, window, cx);
                    }
                });
            }
        }
        MenuCommand::OpenProject(root) => {
            match AppState::find_window(cx, |ws, _| ws.session_for_project(&root).is_some()) {
                Some(owner) => {
                    AppState::with_workspace(&owner, cx, |ws, window, cx| {
                        crate::background::activate_window(window, cx);
                        if let Some(ix) = ws.session_for_project(&root) {
                            ws.activate(ix, window, cx);
                        }
                    });
                }
                None => {
                    with_front_workspace(cx, |ws, window, cx| {
                        ws.spawn_session(None, Some(root), window, cx);
                    });
                }
            }
        }
        MenuCommand::OpenSettings => settings_window::open_settings(None, cx),
        MenuCommand::CheckForUpdates => {
            settings_window::open_settings(Some(settings_window::Section::Updates), cx);
            updates::check_for_updates(true, cx);
        }
    }
}

/// Open Folder…: pick a folder and open a new terminal there, in the window in front.
pub(crate) fn open_folder(cx: &mut App) {
    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some("Open".into()),
    });
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = paths.await else { return };
        let Some(folder) = paths.into_iter().next() else { return };
        cx.update(|cx| {
            with_front_workspace(cx, |ws, window, cx| {
                ws.spawn_session(None, Some(folder), window, cx);
            });
        });
    })
    .detach();
}

/// devctl: the status item's menu as text, as it would be built now.
#[cfg(feature = "devctl")]
pub(crate) fn dump(cx: &App) -> String {
    let snapshot = snapshot(crate::workspace::ambient::terminals(cx), cx);
    let mut out = model::dump(&model::build(&snapshot, Instant::now()));
    out.push_str(&format!("installed={}\n", app(cx).menu.bar.is_some()));
    out
}

/// devctl: the status item's icon in each state, drawn to TIFFs in `dir` with the theme's colours.
#[cfg(feature = "devctl")]
pub(crate) fn save_icons(dir: &Path, prefix: &str, cx: &App) -> anyhow::Result<Vec<PathBuf>> {
    let colors = snapshot(Vec::new(), cx).colors;
    status_item::save_icons(dir, prefix, colors)
}

/// devctl: the Dock's badge, bounces and menu.
#[cfg(feature = "devctl")]
pub(crate) fn dock_dump(cx: &App) -> String {
    let terminals = crate::workspace::ambient::terminals(cx);
    let dock = cx.try_global::<crate::platform::dock::DockState>();
    let mut out = format!(
        "dock badge={:?} bounces={}\n",
        dock.and_then(|d| d.badge.clone()),
        dock.map_or(0, |d| d.bounces)
    );
    for entry in model::dock_menu(&terminals, Instant::now()) {
        out.push_str(&match entry {
            DockEntry::Header(title) => format!("== {title}\n"),
            DockEntry::Terminal { label, tab } => format!("{label}  (tab {tab})\n"),
            DockEntry::Separator => "----\n".to_string(),
            DockEntry::NewTerminal => "New Terminal\n".to_string(),
            DockEntry::NewWindow => "New Window\n".to_string(),
        });
    }
    out
}

fn project_name(root: &Path) -> String {
    root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| root.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::registers;

    /// While the settings window records a new hotkey the current one is held off, so
    /// pressing it again records it instead of opening the quick panel.
    #[test]
    fn a_recording_holds_the_hotkey_off() {
        assert!(registers("alt-space", false));
        assert!(!registers("alt-space", true));
        assert!(!registers("", false), "no hotkey set");
    }
}
