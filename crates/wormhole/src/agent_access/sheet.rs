//! What the question says and how it is put on the screen (`agent-permissions.md`, 7.2): a native sheet on
//! the front window, with the program's facts in a monospace block. The words are in the catalogs; the
//! program's name, path, pid and signer are data.

use std::path::Path;

use gpui_kit::*;
use wormhole_core::config::AgentAccess;
use wormhole_core::ipc::access::{Capability, Kind, Need, Pending, Resolved};
use wormhole_core::platform::codesign::{Signature, SignerKind};
use wormhole_core::platform::paths;
use wormhole_text::keys::{
    AGENT_SHEET_BUTTON_ALWAYS, AGENT_SHEET_BUTTON_DENY, AGENT_SHEET_BUTTON_DONT_KEEP,
    AGENT_SHEET_BUTTON_KEEP, AGENT_SHEET_BUTTON_QUIT, AGENT_SHEET_BUTTON_SESSION, AGENT_SHEET_DETAIL,
    AGENT_SHEET_DETAIL_SCRIPTS, AGENT_SHEET_LABEL_ASKED_BY, AGENT_SHEET_LABEL_PATH, AGENT_SHEET_LABEL_PID,
    AGENT_SHEET_LABEL_RUNS_IN, AGENT_SHEET_LABEL_RUNS_ON, AGENT_SHEET_LABEL_SIGNED_BY,
    AGENT_SHEET_LABEL_STARTED_BY, AGENT_SHEET_SIGNED_AD_HOC, AGENT_SHEET_SIGNED_APPLE,
    AGENT_SHEET_SIGNED_NONE, AGENT_SHEET_SIGNED_OTHER, AGENT_SHEET_SIGNED_UNREADABLE,
    AGENT_SHEET_TITLE_SCRIPTS, AGENT_SHEET_WANTS_CLOSE, AGENT_SHEET_WANTS_OPEN, AGENT_SHEET_WANTS_READ,
    AGENT_SHEET_WANTS_RUN, AGENT_SHEET_WANTS_TYPE, AGENT_SHEET_WARNING_AD_HOC, AGENT_SHEET_WARNING_CHANGED,
    AGENT_SHEET_WARNING_ROOT, AGENT_SHEET_WARNING_UNSIGNED, agent_change_detail, agent_change_title,
    agent_sheet_in_terminal, agent_sheet_runs_on, agent_sheet_signed_developer, agent_sheet_signed_team,
    agent_sheet_title, agent_sheet_warning_unusual,
};
use wormhole_text::keys::{
    SETTINGS_AGENTS_ACCESS_ASK, SETTINGS_AGENTS_ACCESS_OFF, SETTINGS_AGENTS_ACCESS_OPEN,
};
use wormhole_text::text;

use crate::app_state::{AppState, OpenWindow, app};
use crate::platform::sheet::{self, Choice};

/// The answers a question offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Button {
    /// Not now, and not remembered as more than a no (and not asked again for two minutes).
    Deny,
    /// While this program's process runs.
    Session,
    /// Until Wormhole quits.
    Quit,
    /// For good.
    Always,
    /// Keep a mode that was changed outside Wormhole.
    Keep,
    /// Put the earlier mode back.
    DontKeep,
}

impl Button {
    fn label(self) -> &'static str {
        text(match self {
            Button::Deny => AGENT_SHEET_BUTTON_DENY,
            Button::Session => AGENT_SHEET_BUTTON_SESSION,
            Button::Quit => AGENT_SHEET_BUTTON_QUIT,
            Button::Always => AGENT_SHEET_BUTTON_ALWAYS,
            Button::Keep => AGENT_SHEET_BUTTON_KEEP,
            Button::DontKeep => AGENT_SHEET_BUTTON_DONT_KEEP,
        })
    }
}

/// A question as it is shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Display {
    pub title: String,
    pub detail: String,
    pub accessory: String,
    /// In the order they are laid out; the first is the default (Return).
    pub buttons: Vec<Button>,
}

/// The question about a mode that was changed outside Wormhole to something wider than it was.
pub(super) fn change_display(previous: AgentAccess, requested: AgentAccess) -> Display {
    let words = |mode| {
        text(match mode {
            AgentAccess::Off => SETTINGS_AGENTS_ACCESS_OFF,
            AgentAccess::Ask => SETTINGS_AGENTS_ACCESS_ASK,
            AgentAccess::Open => SETTINGS_AGENTS_ACCESS_OPEN,
        })
    };
    Display {
        title: agent_change_title(words(requested)),
        detail: agent_change_detail(words(previous)),
        accessory: String::new(),
        buttons: vec![Button::DontKeep, Button::Keep],
    }
}

/// The answers `pending` offers, the default first. A script is not a program that stays: there is no process
/// to allow "for this session", and "Always" would let anything in for good.
pub(super) fn buttons(pending: &Pending) -> Vec<Button> {
    let resolved = &pending.resolved;
    let mut buttons = vec![Button::Deny];
    if resolved.process.is_some() {
        buttons.push(Button::Session);
    }
    buttons.push(Button::Quit);
    if resolved.identity.can_be_remembered() {
        buttons.push(Button::Always);
    }
    buttons
}

/// What the question says for `pending`.
pub(super) fn display(pending: &Pending, cx: &App) -> Display {
    let resolved = &pending.resolved;
    let scripts = resolved.identity.kind == Kind::Scripts;
    let title = if scripts {
        text(AGENT_SHEET_TITLE_SCRIPTS).to_string()
    } else {
        agent_sheet_title(&resolved.label)
    };
    let wants = text(match pending.need {
        Need::Capability(Capability::Read) => AGENT_SHEET_WANTS_READ,
        Need::Capability(Capability::Type) => AGENT_SHEET_WANTS_TYPE,
        Need::Capability(Capability::Run) => AGENT_SHEET_WANTS_RUN,
        Need::Capability(Capability::Open) => AGENT_SHEET_WANTS_OPEN,
        Need::Capability(Capability::Close) | Need::Approved | Need::Nothing => AGENT_SHEET_WANTS_CLOSE,
    });
    let mut detail =
        format!("{wants}\n\n{}", text(if scripts { AGENT_SHEET_DETAIL_SCRIPTS } else { AGENT_SHEET_DETAIL }));
    for warning in warnings(resolved, cx) {
        detail.push_str("\n\n");
        detail.push_str(&warning);
    }
    let buttons = buttons(pending);
    Display { title, detail, accessory: facts_block(resolved, cx), buttons }
}

fn warnings(resolved: &Resolved, cx: &App) -> Vec<String> {
    let mut lines = Vec::new();
    let program = resolved.program.as_ref();
    if resolved.unusual_signer {
        let name = program
            .and_then(|program| program.path.file_name())
            .map_or_else(|| resolved.label.clone(), |name| name.to_string_lossy().into_owned());
        lines.push(agent_sheet_warning_unusual(&name));
    }
    match resolved.identity.kind {
        Kind::Unsigned => lines.push(text(AGENT_SHEET_WARNING_UNSIGNED).to_string()),
        Kind::AdHoc => lines.push(text(AGENT_SHEET_WARNING_AD_HOC).to_string()),
        Kind::DeveloperId | Kind::Apple | Kind::Script | Kind::Scripts => {}
    }
    if changed_since_allowed(resolved, cx) {
        lines.push(text(AGENT_SHEET_WARNING_CHANGED).to_string());
    }
    if resolved.elevated {
        lines.push(text(AGENT_SHEET_WARNING_ROOT).to_string());
    }
    lines
}

/// Whether the same agent was allowed before as something else: another signer, another build, another
/// install. (Stored approvals only: what "for this session" allowed ended with the process.)
fn changed_since_allowed(resolved: &Resolved, cx: &App) -> bool {
    let Some(agent) = resolved.identity.agent else { return false };
    let key = resolved.identity.key();
    let word = wormhole_core::ipc::access::agent_word(agent);
    let state = app(cx);
    state
        .store_of(state.active_profile())
        .and_then(|store| store.agent_grants().ok())
        .is_some_and(|rows| rows.iter().any(|row| row.key.agent == word && row.key != key))
}

/// The labelled lines under the question: where the program is, who signed it, where it runs.
fn facts_block(resolved: &Resolved, cx: &App) -> String {
    let mut rows: Vec<(&str, String)> = Vec::new();
    let scripts = resolved.identity.kind == Kind::Scripts;
    if let Some(program) = &resolved.program {
        rows.push((
            text(if scripts { AGENT_SHEET_LABEL_ASKED_BY } else { AGENT_SHEET_LABEL_PATH }),
            shown_path(&program.path),
        ));
        rows.push((text(AGENT_SHEET_LABEL_PID), program.pid.to_string()));
        let signer = signer(&program.signature);
        match &program.interpreter {
            Some(interpreter) => {
                rows.push((text(AGENT_SHEET_LABEL_RUNS_ON), agent_sheet_runs_on(interpreter, &signer)));
            }
            None => rows.push((text(AGENT_SHEET_LABEL_SIGNED_BY), signer)),
        }
        if let Some(parent) = parent_name(program.pid) {
            let started_by = resolved.started_by.as_ref().map_or(parent, |other| other.clone());
            rows.push((text(AGENT_SHEET_LABEL_STARTED_BY), started_by));
        }
        if let Some(title) = terminal_of(program.pid, cx) {
            rows.push((text(AGENT_SHEET_LABEL_RUNS_IN), agent_sheet_in_terminal(&title)));
        }
    }
    let width = rows.iter().map(|(label, _)| label.chars().count()).max().unwrap_or(0);
    rows.iter()
        .map(|(label, value)| format!("{label:<width$}  {value}", width = width))
        .collect::<Vec<_>>()
        .join("\n")
}

/// How a signature reads on the sheet.
pub(super) fn signer(signature: &Signature) -> String {
    match signature.kind {
        SignerKind::DeveloperId => match (&signature.subject, &signature.team_id) {
            (Some(subject), Some(team)) => agent_sheet_signed_developer(subject, team),
            (None, Some(team)) => agent_sheet_signed_team(team),
            _ => text(AGENT_SHEET_SIGNED_OTHER).to_string(),
        },
        SignerKind::AppleSystem => text(AGENT_SHEET_SIGNED_APPLE).to_string(),
        SignerKind::AdHoc => text(AGENT_SHEET_SIGNED_AD_HOC).to_string(),
        SignerKind::Other => text(AGENT_SHEET_SIGNED_OTHER).to_string(),
        SignerKind::Unsigned => text(AGENT_SHEET_SIGNED_NONE).to_string(),
        SignerKind::Unreadable => text(AGENT_SHEET_SIGNED_UNREADABLE).to_string(),
    }
}

/// A path as the title row writes it: the home folder as `~`.
pub(super) fn shown_path(path: &Path) -> String {
    match paths::home().as_deref().and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// The program that started `pid`, by name.
fn parent_name(pid: u32) -> Option<String> {
    let parent = wormhole_core::platform::process::parent(pid)?;
    let path = wormhole_core::platform::process::executable(parent)?;
    path.file_name().map(|name| name.to_string_lossy().into_owned())
}

/// The name of the terminal `pid` runs in, when it runs in one of Wormhole's.
fn terminal_of(pid: u32, cx: &App) -> Option<String> {
    let peer = wormhole_core::ipc::Peer::new(
        pid,
        wormhole_core::platform::process::ancestors(pid, wormhole_core::ipc::ANCESTORS_MAX),
    );
    app(cx).windows.iter().find_map(|window| {
        window
            .workspace
            .read_with(cx, |ws, cx| {
                let id = ws.control_terminals_under(&peer, cx).first().copied()?;
                ws.control_terminal(id, cx).map(|terminal| terminal.name)
            })
            .ok()
            .flatten()
    })
}

/// What answers a sheet: a button's place, or nothing when it was withdrawn.
pub(super) type Reply = smol::channel::Receiver<Option<usize>>;

/// Put `display` on `window`; the answer arrives on the channel. A window without a native sheet gets GPUI's
/// prompt, with the facts block in its text.
pub(super) fn show(window: &OpenWindow, display: &Display, cx: &mut App) -> Option<Reply> {
    let labels: Vec<&str> = display.buttons.iter().map(|button| button.label()).collect();
    AppState::with_workspace(window, cx, |_, window, cx| {
        let choice = Choice {
            title: &display.title,
            detail: &display.detail,
            accessory: (!display.accessory.is_empty()).then_some(display.accessory.as_str()),
            buttons: &labels,
        };
        if let Some(native) = sheet::choose(window, &choice) {
            return native;
        }
        let (tx, rx) = smol::channel::bounded(1);
        let buttons: Vec<PromptButton> = labels
            .iter()
            .enumerate()
            .map(
                |(place, label)| {
                    if place == 0 { PromptButton::cancel(*label) } else { PromptButton::new(*label) }
                },
            )
            .collect();
        let detail = format!("{}\n\n{}", display.detail, display.accessory);
        let answer = window.prompt(PromptLevel::Info, &display.title, Some(&detail), &buttons, cx);
        window
            .spawn(cx, async move |_| {
                let _ = tx.send(answer.await.ok()).await;
            })
            .detach();
        rx
    })
}

/// Take the question off the screen without an answer (it timed out, or was answered some other way).
pub(super) fn withdraw(window: &OpenWindow, cx: &mut App) {
    AppState::with_workspace(window, cx, |_, window, _| sheet::dismiss(window));
}
