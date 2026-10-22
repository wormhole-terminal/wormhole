//! The command palette: ⇧⌘P from anywhere, the terminal too. Over the window in front, one
//! search across the terminals of every window, the folders they are in (and recent ones), the
//! commits of the current branch, the settings and the actions, grouped in a fixed order (core's
//! `search`). A prefix or Tab narrows it to one kind: `@` terminals, `~` folders, `#` commits,
//! `,` settings, `>` actions (no Files: that scope is in the backlog). With nothing typed it lists
//! what matters now and the actions that make sense here, each with its key. Esc clears, then
//! closes; the keyboard goes back where it was (`overlay`).
//!
//! What it lists is read outside any window's draw (`gather`), when it opens and again once a
//! second while it is open, as the quick panel does, whose parts it shares (`style::palette_*`).

mod rows;
mod view;

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;
use wormhole_core::git;
use wormhole_core::keymap::palette as palette_actions;
use wormhole_core::project::{ActionKind, Feature};
use wormhole_core::search::{self, Scope};
use wormhole_core::sidebar::display_path;

use super::app::{AppState, OpenWindow, app, open_tab};
use super::overlay::OverlayView;
use super::{Workspace, settings_page};
use crate::settings::settings;
use rows::{FolderItem, Item, ProfileCommand, ProjectAction, ProjectItem, Row, Snapshot, TerminalItem};

pub(crate) use rows::Recent;

pub(crate) struct CommandPalette {
    workspace: WeakEntity<Workspace>,
    /// The palette's window (`AppState`'s id): its terminals have ⌘ keys, its terminal is "here".
    window_id: i64,
    search: Entity<InputState>,
    scope: Scope,
    snapshot: Rc<Snapshot>,
    rows: Rc<Vec<Row>>,
    list: ListState,
    /// The selected row, by what it is (rows come and go as the snapshot changes).
    selected: Option<Item>,
    _subscriptions: Vec<Subscription>,
    _refresh: Task<()>,
    _commits: Option<Task<()>>,
}

/// Results laid out above and below the list's view, so scrolling doesn't pop them in.
const LIST_OVERDRAW: f32 = 200.0;

/// What was chosen in the palette lately (folders, settings, commits), for its Recent group.
#[derive(Default)]
struct Chosen(Vec<Recent>);

impl Global for Chosen {}

/// ⇧⌘P (and View → Command Palette…): open the palette over the window in front, or close it
/// there. Runs outside any window's update (the action handler defers it).
pub(crate) fn toggle(cx: &mut App) {
    let Some(front) = AppState::front_window(cx) else { return };
    let state = front.workspace.read_with(cx, |ws, _| (ws.palette_open(), ws.first_run.is_open()));
    match state {
        // The first-run sheet holds the keyboard until it is done.
        Ok((_, true)) | Err(_) => {}
        Ok((true, false)) => {
            AppState::with_workspace(&front, cx, |ws, window, cx| ws.close_overlay(window, cx));
        }
        Ok((false, false)) => open_in(&front, "", cx),
    }
}

/// Open the palette over `front` with `query` typed (a prefix picks the scope). Outside any
/// window's update.
pub(crate) fn open_in(front: &OpenWindow, query: &str, cx: &mut App) {
    let snapshot = gather(front.id, cx);
    let (window_id, query) = (front.id, query.to_string());
    AppState::with_workspace(front, cx, move |ws, window, cx| {
        crate::background::activate_window(window, cx);
        let return_focus = ws.end_overlay(window, cx);
        let workspace = cx.weak_entity();
        let cwd = ws.active_terminal().read(cx).cwd.clone();
        let palette =
            cx.new(|cx| CommandPalette::new(workspace, window_id, snapshot, cwd, &query, window, cx));
        ws.show_overlay(OverlayView::Palette(palette), return_focus, cx);
    });
}

/// Everything the palette lists, read now: every window's terminals, their folders and the recent
/// ones, the actions the window in front can do, its project's actions and where its terminal is.
fn gather(window_id: i64, cx: &mut App) -> Snapshot {
    let menu = super::app_menu::state(cx);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let key = cx.active_window().map(|w| w.window_id());
    let finished = super::notifications::finished_commands(cx);
    // The shells besides the default: that one is what New terminal opens.
    let default = super::shells::default_key(settings(cx)).to_string();
    let shells =
        wormhole_core::shells::choices(settings(cx)).into_iter().filter(|c| c.key != default).collect();
    // The settings sections this build hides (a Preview section whose flag is off): the palette
    // leaves their rows out, as the settings window's own nav and search do.
    let hidden_sections = wormhole_core::settings_index::SectionId::ALL
        .into_iter()
        .filter(|section| !section.is_visible(settings(cx)))
        .collect();
    let list = &app(cx).profiles;
    let profiles = list
        .profiles()
        .iter()
        .map(|p| rows::ProfileChoice { id: p.id.to_string(), name: p.name.clone() })
        .collect();
    let shown_profile = list.active_id().to_string();
    let window_profile = app(cx)
        .windows
        .iter()
        .find(|w| w.id == window_id)
        .map_or_else(|| shown_profile.clone(), |w| w.profile.to_string());
    let mut snapshot = Snapshot {
        actions: palette_actions::actions(&menu),
        shells,
        profiles,
        shown_profile,
        window_profile,
        hidden_sections,
        ..Snapshot::default()
    };
    let mut folders: Vec<FolderItem> = Vec::new();
    for (ix, window) in app(cx).windows.iter().enumerate().filter(|(_, w)| !w.hidden) {
        let own = window.id == window_id;
        let read = window.workspace.read_with(cx, |ws, cx| {
            let facts = ws.tab_facts(key == Some(window.handle.window_id()), cx);
            let numbers = if own { ws.palette_numbers(cx) } else { Vec::new() };
            (facts, numbers, ws.session_folders(cx))
        });
        let Ok((facts, numbers, session_folders)) = read else { continue };
        for facts in &facts {
            let terminal = super::ambient::terminal(facts, finished.and_then(|f| f.get(&facts.tab)));
            let number = numbers.iter().find(|(session, _)| *session == facts.session).map(|(_, n)| *n);
            snapshot.terminals.push(TerminalItem { terminal, window_number: ix + 1, number });
        }
        for path in session_folders {
            match folders.iter_mut().find(|f| f.path == path) {
                Some(folder) => folder.terminals += 1,
                None => folders.push(folder_item(path, 1, home.as_deref())),
            }
        }
    }
    for root in app(cx).menu.recent() {
        if !folders.iter().any(|f| &f.path == root) {
            folders.push(folder_item(root.clone(), 0, home.as_deref()));
        }
    }
    snapshot.folders = folders;
    snapshot.recent = cx.try_global::<Chosen>().map(|c| c.0.clone()).unwrap_or_default();
    let here = app(cx).windows.iter().find(|w| w.id == window_id).cloned();
    if let Some(here) = here {
        let read = here.workspace.read_with(cx, |ws, cx| ws.palette_here(home.as_deref(), cx));
        if let Ok((cwd, changes, project)) = read {
            (snapshot.cwd, snapshot.changes, snapshot.project) = (cwd, changes, project);
        }
    }
    snapshot
}

fn folder_item(path: PathBuf, terminals: usize, home: Option<&Path>) -> FolderItem {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let parent = path.parent().map(|p| display_path(p, home)).unwrap_or_default();
    FolderItem { path, name, parent, terminals }
}

impl Workspace {
    /// The sidebar's ⌘1–9, by session id.
    fn palette_numbers(&self, cx: &App) -> Vec<(i64, u8)> {
        let numbered = self.session_listing(cx).numbered();
        numbered.iter().enumerate().map(|(n, &ix)| (self.sessions[ix].id, n as u8 + 1)).collect()
    }

    /// The folder each terminal is in, as the sidebar groups them: its repository, else the
    /// folder itself.
    fn session_folders(&self, cx: &App) -> Vec<PathBuf> {
        self.sessions
            .iter()
            .filter_map(|session| {
                let cwd = session.terminal().read(cx).cwd.clone()?;
                Some(session.repo_root.of(&cwd))
            })
            .collect()
    }

    /// The first terminal in `folder` (as [`Self::session_folders`] says), if any.
    fn session_in_folder(&self, folder: &Path, cx: &App) -> Option<usize> {
        self.sessions.iter().position(|session| {
            session.terminal().read(cx).cwd.as_deref().is_some_and(|cwd| session.repo_root.of(cwd) == folder)
        })
    }

    /// Where the current terminal is, how many files changed there, and its project's actions
    /// once it is trusted.
    fn palette_here(
        &self,
        home: Option<&Path>,
        cx: &App,
    ) -> (Option<String>, Option<usize>, Option<ProjectItem>) {
        let session = self.active_session();
        let terminal = session.terminal().read(cx);
        let cwd = terminal.cwd.as_deref().map(|cwd| display_path(cwd, home));
        let changes = session
            .git
            .status
            .as_ref()
            .filter(|_| session.git.cwd == terminal.cwd)
            .map(|status| status.files.len())
            .filter(|&n| n > 0);
        let hub = &app(cx).hub;
        let project = self.active_project(cx).filter(|p| hub.enabled(p, Feature::Actions)).map(|project| {
            let root = &project.folder.root;
            let folder = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            ProjectItem {
                name: format!("{folder}/{}", project.folder.dir_name()),
                actions: project
                    .actions()
                    .iter()
                    .map(|action| ProjectAction {
                        id: action.id.clone(),
                        title: action.title.clone(),
                        command: match &action.kind {
                            ActionKind::Run { command, .. } => Some(command.clone()),
                            _ => None,
                        },
                        key: action.key_spec().map(|k| k.glyphs()),
                    })
                    .collect(),
            }
        });
        (cwd, changes, project)
    }
}

impl CommandPalette {
    fn new(
        workspace: WeakEntity<Workspace>,
        window_id: i64,
        snapshot: Snapshot,
        cwd: Option<PathBuf>,
        query: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(Scope::All.placeholder()));
        window.focus(&search.focus_handle(cx), cx);
        let handle = window.window_handle().window_id();
        let palette = cx.weak_entity();
        // Ahead of the search field, which would take ↑ ↓ ⏎ Tab and Esc for itself.
        let keys = cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle().window_id() != handle {
                return;
            }
            let handled = palette
                .update(cx, |palette, cx| palette.on_key(&event.keystroke, window, cx))
                .unwrap_or(false);
            if handled {
                cx.stop_propagation();
            }
        });
        let subscriptions = vec![
            keys,
            cx.subscribe_in(&search, window, |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::Change = event {
                    this.query_changed(window, cx);
                }
            }),
        ];
        let refresh = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let Ok(window_id) = this.read_with(cx, |this, _| this.window_id) else { break };
                let snapshot = cx.update(|cx| gather(window_id, cx));
                if this.update(cx, |this, cx| this.set_snapshot(snapshot, cx)).is_err() {
                    break;
                }
            }
        });
        let mut palette = Self {
            workspace,
            window_id,
            search,
            scope: Scope::All,
            snapshot: Rc::new(snapshot),
            rows: Rc::default(),
            list: ListState::new(0, ListAlignment::Top, px(LIST_OVERDRAW)),
            selected: None,
            _subscriptions: subscriptions,
            _refresh: refresh,
            _commits: None,
        };
        palette.load_commits(cwd, cx);
        palette.set_query(query, window, cx);
        palette
    }

    /// The commits of the branch the palette's terminal (in `cwd`) is on: once, off the UI thread.
    fn load_commits(&mut self, cwd: Option<PathBuf>, cx: &mut Context<Self>) {
        let Some(cwd) = cwd else { return };
        let task = cx.background_executor().spawn(async move {
            let root = git::repo_root(&cwd).ok().flatten()?;
            let commits = git::log(&root, "HEAD", git::LOG_LIMIT).ok()?;
            Some((root, commits))
        });
        self._commits = Some(cx.spawn(async move |this, cx| {
            let Some((root, commits)) = task.await else { return };
            let _ = this.update(cx, |this, cx| {
                let mut snapshot = (*this.snapshot).clone();
                (snapshot.repo, snapshot.commits) = (Some(root), commits);
                this.snapshot = Rc::new(snapshot);
                this.update_rows(cx);
            });
        }));
    }

    /// A new reading of the terminals and the rest; the commits stay as they were read.
    fn set_snapshot(&mut self, mut snapshot: Snapshot, cx: &mut Context<Self>) {
        snapshot.repo = self.snapshot.repo.clone();
        snapshot.commits = self.snapshot.commits.clone();
        self.snapshot = Rc::new(snapshot);
        self.update_rows(cx);
    }

    fn query(&self, cx: &App) -> String {
        self.search.read(cx).value().to_string()
    }

    /// The field changed: a prefix typed into an unscoped palette narrows it (and leaves the
    /// field), then the rows follow.
    fn query_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.query(cx);
        if self.scope == Scope::All {
            let (scope, rest) = search::parse(&text);
            if scope != Scope::All {
                let rest = rest.to_string();
                self.set_scope(scope, window, cx);
                self.search.update(cx, |search, cx| search.set_value(rest, window, cx));
            }
        }
        self.selected = None;
        self.update_rows(cx);
    }

    /// Put `text` in the field, as if typed (a prefix picks the scope). The field's own
    /// `set_value` doesn't say it changed.
    fn set_query(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let text = text.to_string();
        self.search.update(cx, |search, cx| search.set_value(text, window, cx));
        self.query_changed(window, cx);
    }

    fn set_scope(&mut self, scope: Scope, window: &mut Window, cx: &mut Context<Self>) {
        self.scope = scope;
        self.search.update(cx, |search, cx| search.set_placeholder(scope.placeholder(), window, cx));
        self.selected = None;
        self.update_rows(cx);
    }

    fn update_rows(&mut self, cx: &mut Context<Self>) {
        let rows = rows::rows(&self.snapshot, self.scope, &self.query(cx));
        if *self.rows != rows {
            self.list.reset(rows.len());
            self.rows = Rc::new(rows);
        }
        cx.notify();
    }

    /// The selected row's index: the one chosen, else the first that can be.
    fn selected_ix(&self) -> Option<usize> {
        let chosen =
            self.selected.as_ref().and_then(|item| self.rows.iter().position(|row| row.item() == Some(item)));
        chosen.or_else(|| self.rows.iter().position(Row::selectable))
    }

    fn selected_item(&self) -> Option<Item> {
        self.selected_ix().and_then(|ix| self.rows[ix].item().cloned())
    }

    fn step(&mut self, delta: isize, cx: &mut Context<Self>) {
        let choices: Vec<usize> = (0..self.rows.len()).filter(|&ix| self.rows[ix].selectable()).collect();
        let Some(current) = self.selected_ix().and_then(|ix| choices.iter().position(|&c| c == ix)) else {
            return;
        };
        let next = (current as isize + delta).clamp(0, choices.len() as isize - 1) as usize;
        let ix = choices[next];
        self.selected = self.rows[ix].item().cloned();
        // The label over the first result shows with it.
        let reveal = if next == 0 { 0 } else { ix };
        self.list.scroll_to_reveal_item(reveal);
        cx.notify();
    }

    /// The palette's own keys; `true` when one was used.
    fn on_key(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let m = &keystroke.modifiers;
        let plain = !m.modified();
        let empty = self.query(cx).is_empty();
        match keystroke.key.as_str() {
            // Esc clears, then goes back to everything, then closes.
            "escape" if !empty => self.set_query("", window, cx),
            "escape" if self.scope != Scope::All => self.set_scope(Scope::All, window, cx),
            "escape" => self.close(window, cx),
            "backspace" if plain && empty && self.scope != Scope::All => {
                self.set_scope(Scope::All, window, cx)
            }
            "up" if plain => self.step(-1, cx),
            "down" if plain => self.step(1, cx),
            "tab" if !m.platform && !m.control && !m.alt => {
                let scope = self.scope.step(!m.shift);
                self.set_scope(scope, window, cx);
            }
            "enter" if plain => {
                if let Some(item) = self.selected_item() {
                    self.run(item, window, cx);
                }
            }
            // ⌘↩ opens the result in a new window.
            "enter" if m.platform && !m.shift && !m.alt && !m.control => {
                if let Some(item) = self.selected_item() {
                    self.run_in_new_window(item, window, cx);
                }
            }
            // ⌘C on a setting copies its config key.
            "c" if m.platform && !m.shift && !m.alt && !m.control => match self.selected_item() {
                Some(Item::Setting(id)) => self.copy_key(id, cx),
                _ => return false,
            },
            // In the terminals: ⌘⌫ closes the selected one.
            "backspace" if m.platform && !m.shift => match self.selected_item() {
                Some(Item::Terminal(tab)) => self.close_terminal(tab, window, cx),
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.workspace.update(cx, |ws, cx| ws.close_overlay(window, cx));
    }

    fn copy_key(&self, id: &'static str, cx: &mut Context<Self>) {
        let Some((_, _, entry)) = wormhole_core::settings_index::entry(id) else { return };
        if let Some(key) = entry.keys.first() {
            cx.write_to_clipboard(ClipboardItem::new_string((*key).to_string()));
        }
    }

    /// Carry out a result: the palette closes first, so the keyboard is back where it was and an
    /// action acts there.
    fn run(&mut self, item: Item, window: &mut Window, cx: &mut Context<Self>) {
        remember(&item, cx);
        let snapshot = self.snapshot.clone();
        self.close(window, cx);
        match item {
            Item::Terminal(tab) => cx.defer(move |cx| open_tab(tab, cx)),
            Item::Folder(path) => cx.defer(move |cx| open_folder(path, cx)),
            Item::Commit(sha) => {
                let (Some(root), Some(commit)) = (snapshot.repo.clone(), snapshot.commit(&sha).cloned())
                else {
                    return;
                };
                let _ = self.workspace.update(cx, |ws, cx| ws.reveal_commit(root, commit, cx));
            }
            // The row itself, scrolled to and marked, not a search for its label.
            Item::Setting(id) => cx.defer(move |cx| settings_page::reveal_row(id, cx)),
            Item::Action(id) => {
                if let Some(action) = super::actions::gpui_action(id) {
                    window.dispatch_action(action, cx);
                }
            }
            Item::ProjectAction(id) => {
                let _ = self.workspace.update(cx, |ws, cx| ws.run_project_action(&id, window, cx));
            }
            Item::NewTerminalWith(key) => {
                let _ = self.workspace.update(cx, |ws, cx| ws.new_session_with(Some(key), window, cx));
            }
            Item::Profile(command, id) => {
                let id = SharedString::from(id);
                let action: Box<dyn Action> = match command {
                    ProfileCommand::Switch => Box::new(super::actions::SwitchProfile(id)),
                    ProfileCommand::OpenWindowIn => Box::new(super::actions::OpenWindowInProfile(id)),
                    ProfileCommand::MoveWindowTo => Box::new(super::actions::MoveWindowToProfile(id)),
                };
                window.dispatch_action(action, cx);
            }
        }
    }

    /// ⌘↩: a new window, laid out like this one, with a terminal where the result is: a
    /// terminal's folder, a folder, a commit's repository (with History on the commit there).
    /// A setting opens its own window anyway, and an action acts on the window in front, so
    /// those run as ↩ does.
    fn run_in_new_window(&mut self, item: Item, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = self.snapshot.clone();
        let folder = match &item {
            Item::Terminal(tab) => Folder::OfTab(*tab),
            Item::Folder(path) => Folder::Path(path.clone()),
            Item::Commit(_) => match &snapshot.repo {
                Some(root) => Folder::Path(root.clone()),
                None => return,
            },
            Item::Setting(_)
            | Item::Action(_)
            | Item::ProjectAction(_)
            | Item::NewTerminalWith(_)
            | Item::Profile(..) => {
                return self.run(item, window, cx);
            }
        };
        remember(&item, cx);
        self.close(window, cx);
        let commit = match &item {
            Item::Commit(sha) => snapshot.commit(sha).cloned().map(|commit| (snapshot.repo.clone(), commit)),
            _ => None,
        };
        cx.defer(move |cx| {
            let cwd = match folder {
                Folder::Path(path) => Some(path),
                Folder::OfTab(tab) => AppState::find_window(cx, |ws, _| ws.find_session(tab).is_some())
                    .and_then(|owner| {
                        owner
                            .workspace
                            .read_with(cx, |ws, cx| {
                                let ix = ws.find_session(tab)?;
                                ws.sessions[ix].terminal().read(cx).cwd.clone()
                            })
                            .ok()
                            .flatten()
                    }),
            };
            let cwd = cwd.map(|path| path.to_string_lossy().into_owned());
            AppState::with_front_workspace(cx, |ws, window, cx| ws.new_window_in(cwd, window, cx));
            // The window opens on the next turn and is then the one in front.
            if let Some((Some(root), commit)) = commit {
                cx.defer(move |cx| {
                    AppState::with_front_workspace(cx, |ws, _, cx| ws.reveal_commit(root, commit, cx));
                });
            }
        });
    }

    /// ⌘⌫ on a terminal: close it, from whichever window owns it.
    fn close_terminal(&mut self, tab: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.close(window, cx);
        cx.defer(move |cx| {
            let Some(owner) = AppState::find_window(cx, |ws, _| ws.find_session(tab).is_some()) else {
                return;
            };
            AppState::with_workspace(&owner, cx, |ws, window, cx| {
                let Some(ix) = ws.find_session(tab) else { return };
                crate::background::activate_window(window, cx);
                ws.request_close_session(ix, window, cx);
            });
        });
    }
}

/// Where ⌘↩'s new window starts: a path, or where a terminal is (read once the palette
/// has closed, from the window that owns it).
enum Folder {
    Path(PathBuf),
    OfTab(i64),
}

/// Keep what was chosen for Recent: terminals, folders, settings and commits, in the order
/// they were used; actions are suggested by what fits the window instead.
fn remember(item: &Item, cx: &mut App) {
    let recent = match item {
        Item::Terminal(tab) => Recent::Terminal(*tab),
        Item::Folder(path) => Recent::Folder(path.clone()),
        Item::Setting(id) => Recent::Setting(id),
        Item::Commit(sha) => Recent::Commit(sha.clone()),
        Item::Action(_) | Item::ProjectAction(_) | Item::NewTerminalWith(_) | Item::Profile(..) => return,
    };
    let chosen = &mut cx.default_global::<Chosen>().0;
    chosen.retain(|r| *r != recent);
    chosen.insert(0, recent);
    chosen.truncate(rows::RECENT_MAX);
}

/// A folder: its first terminal, in whichever window; with none there, a new terminal there in the
/// window in front.
fn open_folder(path: PathBuf, cx: &mut App) {
    match AppState::find_window(cx, |ws, cx| ws.session_in_folder(&path, cx).is_some()) {
        Some(owner) => {
            AppState::with_workspace(&owner, cx, |ws, window, cx| {
                crate::background::activate_window(window, cx);
                if let Some(ix) = ws.session_in_folder(&path, cx) {
                    ws.activate(ix, window, cx);
                }
            });
        }
        None => {
            AppState::with_front_workspace(cx, |ws, window, cx| {
                ws.spawn_session(None, Some(path), window, cx)
            });
        }
    }
}

#[cfg(feature = "devctl")]
impl Workspace {
    /// devctl `palette [query]` / `palette close`: open the palette over this window with `query`
    /// typed (a prefix picks the scope); to an open one, type it in place of what is there.
    pub(crate) fn devctl_palette(&mut self, arg: &str, window: &mut Window, cx: &mut Context<Self>) {
        let arg = arg.trim();
        if arg == "close" {
            self.close_overlay(window, cx);
            return;
        }
        if let Some(palette) = self.palette() {
            palette.update(cx, |palette, cx| {
                palette.scope = Scope::All;
                palette.set_query(arg, window, cx);
            });
            return;
        }
        let Some(own) = app(cx).windows.iter().find(|w| w.id == self.window_id).cloned() else { return };
        let query = arg.to_string();
        // After this update: the palette reads every window, this one too.
        cx.defer(move |cx| open_in(&own, &query, cx));
    }
}

#[cfg(all(test, feature = "devctl"))]
mod tests {
    use gpui_kit::{Focusable as _, TestAppContext};

    use super::super::app::OpenWindow;
    use super::super::view_tests::{draw, isolate_in, press, read, start};

    fn key(cx: &mut TestAppContext, window: &OpenWindow, keys: &str) {
        draw(cx, window);
        press(cx, window, keys);
    }

    /// Whether the terminal in front has the keyboard.
    fn terminal_focused(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
        draw(cx, window);
        cx.update(|cx| {
            window.handle.update(cx, |_, win, cx| {
                let workspace = window.workspace.upgrade()?;
                Some(workspace.read(cx).active_terminal().focus_handle(cx).is_focused(win))
            })
        })
        .ok()
        .flatten()
        .unwrap_or(false)
    }

    #[gpui_kit::test]
    fn the_palette_and_the_sheet_open_on_their_keys_and_give_the_keyboard_back(cx: &mut TestAppContext) {
        if isolate_in(
            module_path!(),
            "the_palette_and_the_sheet_open_on_their_keys_and_give_the_keyboard_back",
        ) {
            return;
        }
        let window = start(cx, 2);
        assert!(terminal_focused(cx, &window));

        // ⇧⌘P from the terminal: the palette takes the keyboard.
        key(cx, &window, "shift-cmd-p");
        assert!(read(cx, &window, |ws| ws.palette_open()), "⇧⌘P opens the palette");
        assert!(!terminal_focused(cx, &window), "its search has the keyboard");
        // A prefix narrows it; Esc goes back to everything, then closes.
        key(cx, &window, "@");
        let scope = cx
            .update(|cx| window.workspace.read_with(cx, |ws, cx| ws.palette().map(|p| p.read(cx).scope)))
            .ok()
            .flatten();
        assert_eq!(scope, Some(wormhole_core::search::Scope::Terminals));
        key(cx, &window, "escape");
        assert!(read(cx, &window, |ws| ws.palette_open()), "the first Esc leaves the scope");
        key(cx, &window, "escape");
        assert!(!read(cx, &window, |ws| ws.palette_open()), "the second closes it");
        assert!(terminal_focused(cx, &window), "and the keyboard is back in the terminal");

        // ⌥⌘/ opens the sheet; ⇧⌘P again over the palette closes it.
        key(cx, &window, "alt-cmd-/");
        assert!(read(cx, &window, |ws| ws.sheet_open()), "⌥⌘/ opens the shortcut sheet");
        key(cx, &window, "escape");
        assert!(!read(cx, &window, |ws| ws.sheet_open()));
        assert!(terminal_focused(cx, &window));
        key(cx, &window, "shift-cmd-p");
        key(cx, &window, "shift-cmd-p");
        assert!(!read(cx, &window, |ws| ws.palette_open()), "⇧⌘P toggles");
        assert!(terminal_focused(cx, &window));

        // Running an action: the palette closes first and the action acts on the window.
        key(cx, &window, "shift-cmd-p");
        for c in [">", "n", "e", "w", "space", "t", "e", "r", "m"] {
            key(cx, &window, c);
        }
        key(cx, &window, "enter");
        read(cx, &window, |ws| {
            assert!(!ws.palette_open());
            assert_eq!(ws.sessions.len(), 3, "New terminal ran");
        });
    }
}
