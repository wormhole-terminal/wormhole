//! The harness of the whole-window tests: a real `Workspace` (with real shells) in GPUI's test platform,
//! opened through the same `open_workspace_window` the app uses.
//!
//! Opening one reads and writes the data dir and starts shells, so each test runs in a child
//! process of the test binary with a scratch data dir and `/bin/cat` for a shell (see
//! [`isolate_in`]): the environment can't be changed from inside a test process, and a test must
//! never touch the user's real data. The cases are in the `*_tests` files beside it.

use std::process::Command;

use gpui_kit::accesskit::{Action, ActionRequest, NodeId, Role, TreeId, TreeUpdate};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{App, Context, Keystroke, TestAppContext, Window};
use wormhole_core::agents::AgentId;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};
use wormhole_core::view::sidebar::SidebarNumbers;

use crate::app_state::{AppState, OpenWindow, app};
use crate::notifications;
use crate::settings::SettingsStore;
use crate::window::Workspace;
use crate::window::window_lifecycle::open_workspace_window;

const CHILD: &str = "WORMHOLE_VIEW_TEST_CHILD";

/// [`isolate`] for a test of another module (`module_path!()` there).
pub(crate) fn isolate_in(module_path: &str, name: &str) -> bool {
    if std::env::var_os(CHILD).is_some() {
        // The marker alone must never let a test run on the person's own data: set by hand, it once
        // rewrote a real config.json. Only the scratch folder the parent made counts.
        let data = std::env::var_os("WORMHOLE_DATA_DIR").map(std::path::PathBuf::from);
        assert!(
            data.is_some_and(|dir| dir.starts_with(std::env::temp_dir())
                && dir.file_name().is_some_and(|n| n.to_string_lossy().starts_with("wormhole-view-test-"))),
            "{CHILD} is set without a scratch WORMHOLE_DATA_DIR from the test harness; run the test without it"
        );
        return false;
    }
    // The test's path as the harness names it: without the crate's name.
    let module = module_path.split_once("::").map_or("", |(_, rest)| rest);
    let test = format!("{module}::{name}");
    let data = std::env::temp_dir().join(format!("wormhole-view-test-{}-{name}", std::process::id()));
    // A home of its own too: terminals start there, git reads its config from there, and Settings
    // looks for the agents' configs under it.
    let home = data.join("home");
    std::fs::create_dir_all(&home).expect("creates the scratch data dir");
    let exe = std::env::current_exe().expect("finds the test binary");
    let mut command = Command::new(exe);
    command
        .args([&test, "--exact", "--test-threads=1"])
        .current_dir(&data)
        .env(CHILD, "1")
        .env("HOME", &home)
        .env("WORMHOLE_DATA_DIR", &data)
        .env("WORMHOLE_SHELL", "/bin/cat")
        .env("WORMHOLE_AGENT_HOME", data.join("agent-home"))
        // No agent is installed, whatever the Mac running the tests has on its `PATH`: what Settings
        // and the first run draw for an installed one would differ from machine to machine.
        .env("WORMHOLE_AGENT_PATH", "")
        // Where Settings → Command line tool would put `wh`, and what it reads to say whether it is
        // there: never /usr/local/bin, never the person's shell.
        .env("WORMHOLE_CLI_DIR", data.join("cli-bin"))
        // Where other terminals keep their settings is asked of these (the Import page reads them):
        // inside the scratch folder, never the person's own ~/.config.
        .env("XDG_CONFIG_HOME", home.join(".config"));
    // Run from a Wormhole tab (or its dev instance), the tests would otherwise find its socket,
    // its devctl folder (and take the commands meant for it), its scripted answers and where it
    // starts terminals.
    for var in [
        "WORMHOLE_START_DIR",
        "WORMHOLE_DEVCTL",
        "WORMHOLE_SOCKET",
        "WORMHOLE_TAB_ID",
        "WORMHOLE_BIN",
        // A tab's variables under their old names, which the app reads when the new ones are missing.
        "ANDROMEDA_SOCKET",
        "ANDROMEDA_TAB_ID",
        "ANDROMEDA_BIN",
        "WORMHOLE_AUTH",
        "WORMHOLE_NOTIFY",
        // The other places a terminal's settings are looked for (`import::Env::from_process`).
        "XDG_CONFIG_DIRS",
        "KITTY_CONFIG_DIRECTORY",
        "WEZTERM_CONFIG_FILE",
    ] {
        command.env_remove(var);
    }
    let output = command.output().expect("runs the test binary");
    std::fs::remove_dir_all(&data).ok();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // A filter that matched nothing also exits successfully: make sure the test really ran.
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{test} failed in its isolated process:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

/// An app with one window of `sessions` sessions (terminals), ids 1..
pub(crate) fn start(cx: &mut TestAppContext, sessions: i64) -> OpenWindow {
    let record = WindowRecord {
        id: 1,
        sessions: (1..=sessions).map(|id| SessionRecord::new(id, None, None)).collect(),
        ..Default::default()
    };
    start_with(cx, record)
}

/// Every window runs real terminals, whose PTY reader threads wake the window whenever their program
/// prints: the echo of a key typed into `/bin/cat`, a terminal hanging up as it closes. GPUI's test
/// scheduler reports a wake from another thread as a test that is not deterministic, which failed tests
/// now and then on a loaded runner (issue #3). Allowing parking once turns off the scheduler's
/// cross-thread check, for every thread, for the rest of the test; timers stay deterministic (forbidding
/// parking again keeps idle waits jumping to the next timer) and a hang still panics.
fn let_programs_print(cx: &TestAppContext) {
    cx.executor().allow_parking();
    cx.executor().forbid_parking();
}

/// The globals every window reads besides the settings: the shared state, the updater and the feature
/// table, all of which `app::launch` sets. A test that opens a window sets them here, so none can lack
/// one and a test window has the features the app's has ([`start_with_table`] swaps the table).
pub(crate) fn set_shared_state(cx: &mut App) {
    cx.set_global(AppState::for_test(100));
    cx.set_global(crate::update::Updates::new(None, None));
    crate::features::install(cx);
    crate::window::feature::FeatureTable::profile_shown(cx);
}

/// [`start`] with the window's sessions as `record` has them (a terminal's scrollback, say).
pub(crate) fn start_with(cx: &mut TestAppContext, record: WindowRecord) -> OpenWindow {
    start_with_table(cx, record, crate::features::TABLE)
}

/// [`start_with`] for an app whose feature table is `table`: what the window copies from the global
/// the app installs at launch.
pub(crate) fn start_with_table(
    cx: &mut TestAppContext,
    record: WindowRecord,
    table: &'static [crate::window::feature::FeatureDef],
) -> OpenWindow {
    let_programs_print(cx);
    cx.update(|cx| {
        gpui_kit::init(cx);
        // The test platform delivers notifications only to an app with an identity.
        cx.set_app_identity("app.getwormhole.terminal.test", "Wormhole");
        // Layout tests measure rows and then press them: a list that slides to a new order for 180 ms
        // of the wall clock (not the test clock) would be read halfway, and pressed somewhere else.
        cx.set_reduce_motion(true);
        cx.set_global(SettingsStore::load());
        set_shared_state(cx);
        crate::window::feature::FeatureTable::install(table, cx);
        // These tests were written for the numbers that follow the list; the ones that stay with the
        // terminal have their own (`stable_list_tests`), which switch back with `use_stable`.
        crate::settings::update_settings(cx, |settings| {
            settings.sidebar_numbers = SidebarNumbers::List;
        });
        crate::app::handlers::init(cx);
        notifications::init(cx);
        open_workspace_window(Some(record), None, cx);
    });
    cx.run_until_parked();
    cx.update(|cx| app(cx).windows.first().cloned()).expect("the window opened")
}

/// The numbers that stay with a terminal (the default).
pub(super) fn use_stable(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.sidebar_numbers = SidebarNumbers::Terminal;
        });
    });
}

/// Run `f` on the window's workspace, then let everything it set in motion finish.
pub(crate) fn drive<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R,
) -> R {
    let out = cx.update(|cx| AppState::with_workspace(window, cx, f)).expect("the window is open");
    cx.run_until_parked();
    out
}

/// Draw the window once, as a frame would (not from inside an update of the workspace).
///
/// First as the display would, replaying what GPUI keeps: the chrome's check (`regions::oracle`) panics there
/// when a region shows what the window no longer holds, so every test that draws is a check for a missing
/// notify. Then with every cache dropped, which is what most tests want to read.
pub(crate) fn draw(cx: &mut TestAppContext, window: &OpenWindow) {
    draw_replaying(cx, window);
    draw_fresh(cx, window);
}

/// [`draw`] without the replayed frame before: one frame, every cache dropped. For a test that counts what
/// a frame costs.
pub(crate) fn draw_fresh(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.update(|cx| window.handle.update(cx, |_, window, cx| window.render_frame(cx)))
        .expect("the window is open");
}

/// Draw the window once as the display would: views GPUI keeps are replayed unless they were notified.
/// [`draw`] drops every cache first, which is what most tests want and a test of what a frame leaves
/// alone cannot have.
pub(crate) fn draw_replaying(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.update(|cx| window.handle.update(cx, |_, window, cx| window.draw(cx).clear(cx)))
        .expect("the window is open");
}

/// Read the window's sidebar (its list's state), which is an entity of its own.
pub(crate) fn sidebar_of<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&crate::sidebar::Sidebar) -> R,
) -> R {
    cx.update(|cx| window.workspace.read_with(cx, |ws, cx| f(ws.sidebar.read(cx))))
        .expect("the window is open")
}

/// [`read`] with the app too, for what lives in an entity beside the workspace.
pub(crate) fn read_cx<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&Workspace, &App) -> R,
) -> R {
    cx.update(|cx| window.workspace.read_with(cx, |ws, cx| f(ws, cx))).expect("the window is open")
}

pub(crate) fn read<R>(cx: &mut TestAppContext, window: &OpenWindow, f: impl FnOnce(&Workspace) -> R) -> R {
    cx.update(|cx| window.workspace.read_with(cx, |ws, _| f(ws))).expect("the window is open")
}

/// Run `f` on the window itself (not its workspace).
pub(crate) fn with_window<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&mut Window, &mut App) -> R,
) -> R {
    cx.update(|cx| window.handle.update(cx, |_, window, cx| f(window, cx))).expect("the window is open")
}

/// An agent's hook for session 2 (a background session), as the hook socket delivers it.
pub(crate) fn hook(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    tab: i64,
    event: &str,
    message: Option<&str>,
) {
    drive(cx, window, |ws, window, cx| {
        ws.handle_hook(&tab.to_string(), AgentId::Claude, event, message.map(str::to_string), window, cx)
    });
}

/// Press a key combination in the window, as the keyboard would (`cmd-1`).
pub(crate) fn press(cx: &mut TestAppContext, window: &OpenWindow, keys: &str) {
    let keystroke = Keystroke::parse(keys).expect("a keystroke");
    cx.update(|cx| window.handle.update(cx, |_, window, cx| window.dispatch_keystroke(keystroke, cx)))
        .expect("the window is open");
    cx.run_until_parked();
}

/// Which window GPUI runs `window`'s workspace in (its tick, deferred work and subscriptions).
pub(crate) fn own_window(cx: &mut TestAppContext, window: &OpenWindow) -> Option<gpui_kit::WindowId> {
    let workspace = window.workspace.upgrade().expect("the window is open");
    cx.update(|cx| cx.with_window(workspace.entity_id(), |window, _| window.window_handle().window_id()))
}

/// Say an assistive app has connected to the window (or left it), as the platform's adapter would: the test
/// platform has none. The next frame builds the accessibility tree (or stops).
pub(crate) fn simulate_tree(cx: &mut TestAppContext, window: &OpenWindow, on: bool) {
    with_window(cx, window, |window, _| window.simulate_a11y_active(on));
}

/// Rebuild the whole window as the first frame with the tree does: GPUI's `refresh` replays nothing. The
/// tree this leaves is the reference that frames replaying the chrome must match.
pub(crate) fn rebuild(cx: &mut TestAppContext, window: &OpenWindow) {
    with_window(cx, window, |window, _| window.refresh());
    draw_replaying(cx, window);
}

/// One node of the accessibility tree, as a test compares them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Node {
    pub id: u64,
    pub role: Role,
    pub label: Option<String>,
    pub children: Vec<u64>,
}

/// What GPUI last sent as the window's accessibility tree: every node by id, the node that has focus, and the
/// ways the tree is broken (`problems`, empty for a good one). Two trees are equal when they hold the same
/// nodes with the same links, labels and focus, so a replayed frame is compared with a rebuilt one by `==`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Tree {
    pub root: u64,
    pub focus: u64,
    /// Sorted by id.
    pub nodes: Vec<Node>,
    pub problems: Vec<String>,
}

impl Tree {
    fn of(update: &TreeUpdate) -> Self {
        let root = update.tree.as_ref().map_or(0, |tree| tree.root.0);
        let mut nodes: Vec<Node> = update
            .nodes
            .iter()
            .map(|(id, node)| Node {
                id: id.0,
                role: node.role(),
                label: node.label().map(String::from),
                children: node.children().iter().map(|child| child.0).collect(),
            })
            .collect();
        nodes.sort_by_key(|node| node.id);
        let mut tree = Self { root, focus: update.focus.0, nodes, problems: Vec::new() };
        tree.check();
        tree
    }

    /// Every child link points at a node of the tree, and every node is reached from the root exactly once:
    /// a node under two parents, or listed twice, is as wrong as one that fell out of the tree.
    fn check(&mut self) {
        let mut reached = std::collections::HashMap::<u64, usize>::new();
        let mut pending = vec![self.root];
        *reached.entry(self.root).or_default() += 1;
        while let Some(id) = pending.pop() {
            let Some(node) = self.nodes.iter().find(|node| node.id == id) else {
                self.problems.push(format!("a child link points at {id}, which is not in the tree"));
                continue;
            };
            for &child in &node.children {
                let seen = reached.entry(child).or_default();
                *seen += 1;
                if *seen == 1 {
                    pending.push(child);
                }
            }
        }
        for node in &self.nodes {
            match reached.get(&node.id) {
                None => self
                    .problems
                    .push(format!("{:?} {:?} ({}) is not reached", node.role, node.label, node.id)),
                Some(1) => {}
                Some(n) => self
                    .problems
                    .push(format!("{:?} {:?} ({}) is reached {n} times", node.role, node.label, node.id)),
            }
        }
        let mut ids: Vec<u64> = self.nodes.iter().map(|node| node.id).collect();
        ids.dedup();
        if ids.len() != self.nodes.len() {
            self.problems.push("a node is in the tree twice".into());
        }
    }

    /// The tree is a tree: nothing dangling, unreachable or reached twice.
    pub fn whole(&self) -> bool {
        self.problems.is_empty()
    }

    /// How many nodes have `role`.
    pub fn count(&self, role: Role) -> usize {
        self.nodes.iter().filter(|node| node.role == role).count()
    }

    /// The nodes with `role` whose label starts with `label`.
    pub fn labelled(&self, role: Role, label: &str) -> Vec<&Node> {
        self.nodes
            .iter()
            .filter(|node| node.role == role && node.label.as_deref().is_some_and(|l| l.starts_with(label)))
            .collect()
    }

    /// The one node with `role` whose label starts with `label`.
    pub fn one(&self, role: Role, label: &str) -> u64 {
        match self.labelled(role, label).as_slice() {
            [node] => node.id,
            found => panic!("expected one {role:?} labelled {label:?}, found {found:?} in {:?}", self.nodes),
        }
    }
}

/// The tree of the last frame that built one (`simulate_tree`).
pub(crate) fn tree(cx: &mut TestAppContext, window: &OpenWindow) -> Tree {
    let update =
        with_window(cx, window, |window, _| window.last_a11y_tree_update()).expect("a frame built the tree");
    Tree::of(&update)
}

/// Do what an assistive app does to node `node` of the tree: press it, as VoiceOver's "activate" does.
pub(crate) fn activate(cx: &mut TestAppContext, window: &OpenWindow, node: u64) {
    with_window(cx, window, |window, cx| {
        window.simulate_a11y_action(
            ActionRequest {
                action: Action::Click,
                target_tree: TreeId::ROOT,
                target_node: NodeId(node),
                data: None,
            },
            cx,
        )
    });
    cx.run_until_parked();
}
