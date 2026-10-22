//! The hotkey quick panel: a floating panel over whatever app is in front, without
//! bringing Wormhole forward. It lists the status item's sections (Needs you, Working,
//! Recent) and a Do section; typing filters them. On a waiting row ⌘⏎ opens Reply (typed into
//! that terminal, then Return); ⏎ opens the terminal. Numbered answers wait for agents to report
//! their options (backlog). It is a sibling of the command palette: the same surface,
//! search row, labels, results and footer (`style::palette_*`).
//!
//! The window is GPUI's `PopUp` kind: a non-activating panel that can take the keyboard while
//! another app stays active. It sits 20 % from the top of the screen with the pointer, and goes
//! when it loses the keyboard (a click elsewhere) or on Esc.

mod rows;
mod view;

use gpui_kit::*;
use wormhole_core::notify::Terminal;

use super::app::AppState;
use crate::menubar::ActionItem;
use crate::style;

pub(crate) use rows::{Do, Row, rows};
pub(crate) use view::QuickPanel;

/// The panel's window while it is open.
#[derive(Default)]
struct Open(Option<AnyWindowHandle>);

impl Global for Open {}

/// The hotkey (with `hotkey_action` "quick_panel"): open the panel; pressed again while it is
/// open, show Wormhole instead, so show and hide stays one more press away.
pub(crate) fn toggle(cx: &mut App) {
    if is_open(cx) {
        close(cx);
        crate::background::activate(cx);
        if let Some(front) = AppState::front_window(cx) {
            let _ = front.handle.update(cx, |_, window, cx| crate::background::activate_window(window, cx));
        }
        return;
    }
    open(cx);
}

pub(crate) fn is_open(cx: &App) -> bool {
    cx.try_global::<Open>().is_some_and(|open| open.0.is_some())
}

/// The panel's window, if open (devctl takes its picture).
#[cfg(feature = "devctl")]
pub(crate) fn window(cx: &App) -> Option<AnyWindowHandle> {
    cx.try_global::<Open>().and_then(|open| open.0)
}

pub(crate) fn close(cx: &mut App) {
    let Some(handle) = cx.default_global::<Open>().0.take() else { return };
    // Deferred: this may run inside the panel's own update.
    cx.defer(move |cx| {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    });
}

/// Open the panel (devctl; the hotkey goes through [`toggle`]).
pub(crate) fn open(cx: &mut App) {
    if is_open(cx) {
        return;
    }
    let display = pointer_display(cx);
    let screen = display.as_ref().map(|d| d.bounds().size).unwrap_or_else(|| size(px(1440.), px(900.)));
    let height = view::height_for(&rows(&terminals_now(cx), &do_items(cx), ""));
    let origin = point((screen.width - px(style::QUICK_PANEL_W)) / 2.0, screen.height * style::QP_TOP);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin,
            size: size(px(style::QUICK_PANEL_W), px(height)),
        })),
        titlebar: None,
        focus: true,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        display_id: display.map(|d| d.id()),
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    match crate::background::open_window(options, cx, |window, cx| {
        cx.new(|cx| QuickPanel::new(height, window, cx))
    }) {
        Ok((handle, _)) => cx.default_global::<Open>().0 = Some(handle),
        Err(err) => tracing::error!("opening the quick panel failed: {err:#}"),
    }
}

/// The display the pointer is on (the panel opens there, as Spotlight does).
fn pointer_display(cx: &App) -> Option<std::rc::Rc<dyn PlatformDisplay>> {
    if let Some(id) = crate::platform::panel::pointer_display_id() {
        let found = cx.displays().into_iter().find(|d| u64::from(d.id()) == id);
        if found.is_some() {
            return found;
        }
    }
    cx.primary_display()
}

/// Every terminal of every window, now.
fn terminals_now(cx: &App) -> Vec<Terminal> {
    super::ambient::terminals(cx)
}

/// The Do section: a new terminal where the one in front is, Open Folder…, and the actions of
/// the projects that have a terminal open.
fn do_items(cx: &App) -> Vec<Do> {
    let cwd = AppState::front_window(cx).and_then(|front| {
        front.workspace.read_with(cx, |ws, cx| ws.active_terminal().read(cx).cwd.clone()).ok().flatten()
    });
    let mut items = vec![Do::NewTerminal { cwd }, Do::OpenFolder];
    items.extend(super::menu_bar::project_actions(cx).into_iter().map(Do::Action));
    items
}

/// A terminal's status glyph in a list of every window's terminals (the quick panel, the command
/// palette): the state an agent or a program that reports gives, or a plain terminal's bell,
/// command or prompt.
pub(crate) fn glyph_of(terminal: &Terminal) -> crate::style::StatusGlyph {
    use crate::style::StatusGlyph;
    use wormhole_core::notify::State;
    match (terminal.state, terminal.source.told()) {
        (State::NeedsYou, true) => StatusGlyph::NeedsYou,
        (State::NeedsYou, false) => StatusGlyph::Bell,
        (State::Working, true) => StatusGlyph::Working,
        (State::Working, false) => StatusGlyph::Running,
        (State::Done, _) => StatusGlyph::Done,
        (State::Failed, _) => StatusGlyph::Failed,
        (State::Idle, true) | (State::Exited, _) => StatusGlyph::Idle,
        (State::Idle, false) => StatusGlyph::Shell,
    }
}

/// Carry out a row (⏎ or a click), after closing the panel.
fn run(row: &Row, cx: &mut App) {
    use crate::menubar::MenuCommand;
    let command = match row {
        Row::Terminal(terminal) => MenuCommand::OpenTab(terminal.tab),
        Row::Do(Do::NewTerminal { .. }) => MenuCommand::NewSession,
        Row::Do(Do::OpenFolder) => MenuCommand::OpenFolder,
        Row::Do(Do::Action(ActionItem { root, id, .. })) => {
            MenuCommand::RunAction { root: root.clone(), id: id.clone() }
        }
        Row::Label { .. } | Row::Note(_) => return,
    };
    close(cx);
    cx.defer(move |cx| super::menu_bar::run_command(command, cx));
}
