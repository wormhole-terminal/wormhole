//! Terminals, splits, notes, tasks and windows.

use anyhow::{Result, anyhow};
use gpui_kit::*;
use wormhole_core::agents::AgentId;

use super::{Ctx, nth};

/// A new terminal, where the selected one is or in the folder given (`~/` is the home folder).
pub(super) fn new(ctx: &mut Ctx, arg: &str) -> Result<String> {
    match arg.trim() {
        "" => ctx.workspace.new_session(ctx.window, ctx.cx),
        dir => ctx.workspace.new_session_in(Some(expand_home(dir).into()), ctx.window, ctx.cx),
    }
    Ok(String::new())
}

/// `dir` with a leading `~/` as the home folder.
pub(super) fn expand_home(dir: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    dir.strip_prefix("~/").map_or_else(|| dir.to_string(), |rest| format!("{home}/{rest}"))
}

/// `new-shell <key> [dir]`: a new terminal running the shell `key` names (`shells` lists the
/// keys; `-` is the login shell), in `dir` or where the selected one is.
pub(super) fn new_shell(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let (key, dir) = shell_arg(arg)?;
    ctx.workspace.devctl_new_shell(&key, dir.as_deref().map(expand_home), ctx.window, ctx.cx);
    Ok(String::new())
}

/// `shells`: the shells on offer, one a line: key (`-` for the login shell), name and what runs,
/// tab-separated, and `default` after the one a plain + opens.
pub(super) fn shells(ctx: &mut Ctx, _: &str) -> Result<String> {
    let settings = crate::settings::settings(ctx.cx);
    let default = settings.default_shell.trim();
    Ok(wormhole_core::shells::choices(settings)
        .iter()
        .map(|choice| {
            let key = if choice.key.is_empty() { "-" } else { choice.key.as_str() };
            let marked = if choice.key == default { "\tdefault" } else { "" };
            format!("{key}\t{}\t{}{marked}\n", choice.name, choice.command_line)
        })
        .collect())
}

/// `<key> [dir]`: the shell's key (`-` spells the login shell's empty key) and a folder.
fn shell_arg(arg: &str) -> Result<(String, Option<String>)> {
    let arg = arg.trim();
    let (key, dir) = arg.split_once(' ').unwrap_or((arg, ""));
    if key.is_empty() {
        return Err(anyhow!("expected <key> [dir] (`shells` lists the keys; `-` is the login shell)"));
    }
    let key = if key == "-" { String::new() } else { key.to_string() };
    let dir = dir.trim();
    Ok((key, (!dir.is_empty()).then(|| dir.to_string())))
}

/// Name the `n`th terminal (as `state` numbers them) as a project's layout would: `rename <n> <name>`.
pub(super) fn rename(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let (n, name) = arg.trim().split_once(' ').ok_or_else(|| anyhow!("expected <n> <name>"))?;
    ctx.workspace.devctl_rename(nth(n), name.trim(), ctx.cx);
    Ok(String::new())
}

pub(super) fn activate(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.activate(nth(arg), ctx.window, ctx.cx);
    Ok(String::new())
}

pub(super) fn menu_new_window(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.window.dispatch_action(Box::new(crate::workspace::NewWindow), ctx.cx);
    Ok(String::new())
}

pub(super) fn menu_updates(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.window.dispatch_action(Box::new(crate::workspace::CheckForUpdates), ctx.cx);
    Ok(String::new())
}

pub(super) fn new_window(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.new_window(ctx.window, ctx.cx);
    Ok(String::new())
}

pub(super) fn close_window(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.close_window(ctx.window, ctx.cx);
    Ok(String::new())
}

/// `rows`: the sidebar's list as it is drawn, one a line, with each row's marks.
pub(super) fn rows(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(ctx.workspace.devctl_rows(ctx.cx))
}

/// The terminal `n` (as `state` numbers them) names, which has to exist.
fn row_arg(ctx: &Ctx, n: &str) -> Result<usize> {
    let ix: usize = n.trim().parse::<usize>().map_err(|_| anyhow!("expected a row number, got {n:?}"))?;
    match ix.checked_sub(1).filter(|&ix| ix < ctx.workspace.devctl_session_count()) {
        Some(ix) => Ok(ix),
        None => Err(anyhow!("no row {ix} (`rows` lists them)")),
    }
}

/// `split-join <row> <pane> <left|right|top|bottom>`: the terminal in that row joins the split of
/// the terminal `pane` (its id; made if it stands alone), on that side of its pane, as a row
/// dropped on a pane's edge does. It leaves the split it was in.
pub(super) fn split_join(ctx: &mut Ctx, arg: &str) -> Result<String> {
    use wormhole_core::panes::Zone;
    let mut parts = arg.split_whitespace();
    let (Some(row), Some(pane), Some(zone), None) = (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(anyhow!("expected <row> <pane> <left|right|top|bottom>"));
    };
    let ix = row_arg(ctx, row)?;
    let pane: i64 =
        pane.parse().map_err(|_| anyhow!("expected a pane id (`state` lists them), got {pane:?}"))?;
    let zone = match zone {
        "left" => Zone::Left,
        "right" => Zone::Right,
        "top" => Zone::Top,
        "bottom" => Zone::Bottom,
        other => return Err(anyhow!("expected left|right|top|bottom, got {other:?}")),
    };
    ctx.workspace
        .join_split(ix, pane, zone, ctx.window, ctx.cx)
        .map(|()| String::new())
        .map_err(|error| anyhow!("refused: {error:?}"))
}

/// `split-leave <row>`: the terminal in that row leaves its split and stands alone; the split
/// dissolves when one is left.
pub(super) fn split_leave(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let ix = row_arg(ctx, arg)?;
    match ctx.workspace.leave_split(ix, ctx.cx) {
        true => Ok(String::new()),
        false => Err(anyhow!("row {} is in no split", ix + 1)),
    }
}

/// `row-drop <row> <onto> <row|before|after>`: drop the terminal in the first row on the middle
/// of the second row's (`row`: it joins its split), its top quarter (`before`) or its bottom
/// quarter (`after`), as a drag in the sidebar would. Prints what it did.
pub(super) fn row_drop(ctx: &mut Ctx, arg: &str) -> Result<String> {
    use wormhole_core::sidebar::{Part, RowDrop};
    let mut parts = arg.split_whitespace();
    let (Some(from), Some(onto), Some(part), None) = (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(anyhow!("expected <row> <onto> <row|before|after>"));
    };
    let (from, onto) = (row_arg(ctx, from)?, row_arg(ctx, onto)?);
    let part = match part {
        "row" => Part::Middle,
        "before" => Part::Top,
        "after" => Part::Bottom,
        other => return Err(anyhow!("expected row|before|after, got {other:?}")),
    };
    let (dragged, onto) = (ctx.workspace.devctl_session_id(from), ctx.workspace.devctl_session_id(onto));
    let done = ctx
        .workspace
        .row_drop(dragged, onto, part, ctx.window, ctx.cx)
        .map_err(|error| anyhow!("refused: {error:?}"))?;
    Ok(match done {
        None => "nothing to do\n".to_string(),
        Some(RowDrop::Reorder { to }) => format!("reorder to {}\n", to + 1),
        Some(RowDrop::Join { target, .. }) => format!("join row {}\n", target + 1),
        Some(RowDrop::Leave { to }) => format!("leave to {}\n", to + 1),
    })
}

/// `pane-split <right|down>`: split the focused pane of the selected terminal (⌘D / ⇧⌘D).
pub(super) fn pane_split(ctx: &mut Ctx, arg: &str) -> Result<String> {
    use wormhole_core::panes::Axis;
    let axis = match arg.trim() {
        "right" => Axis::Row,
        "down" => Axis::Column,
        other => return Err(anyhow!("expected right|down, got {other:?}")),
    };
    ctx.workspace.split_pane(axis, ctx.window, ctx.cx);
    Ok(String::new())
}

/// `pane-close [id]`: close that pane, or the focused one.
pub(super) fn pane_close(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let id = pane_id(ctx, arg)?;
    ctx.workspace.close_pane(id, ctx.window, ctx.cx);
    Ok(String::new())
}

pub(super) fn pane_zoom(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.toggle_zoom(ctx.cx);
    Ok(String::new())
}

/// `pane-focus <id|left|right|up|down>`.
pub(super) fn pane_focus(ctx: &mut Ctx, arg: &str) -> Result<String> {
    use wormhole_core::panes::Direction;
    let direction = match arg.trim() {
        "left" => Some(Direction::Left),
        "right" => Some(Direction::Right),
        "up" => Some(Direction::Up),
        "down" => Some(Direction::Down),
        _ => None,
    };
    match direction {
        Some(direction) => ctx.workspace.focus_pane_toward(direction, ctx.window, ctx.cx),
        None => {
            let id = pane_id(ctx, arg)?;
            ctx.workspace.focus_pane(id, ctx.window, ctx.cx);
        }
    }
    Ok(String::new())
}

/// `layout <columns|rows|grid|single>`: View → Layout on the selected terminal's split.
pub(super) fn layout(ctx: &mut Ctx, arg: &str) -> Result<String> {
    use wormhole_core::panes::Preset;
    let preset = match arg.trim() {
        "columns" => Preset::TwoColumns,
        "rows" => Preset::TwoRows,
        "grid" => Preset::Grid,
        "single" => Preset::Single,
        other => return Err(anyhow!("expected columns|rows|grid|single, got {other:?}")),
    };
    ctx.workspace.apply_layout(preset, ctx.window, ctx.cx);
    Ok(String::new())
}

/// `panes`: the selected split's panes and dividers with their rectangles.
pub(super) fn panes(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(ctx.workspace.devctl_panes())
}

/// A pane's id from `arg`, or the focused pane's without one; an id no pane of this window has
/// is an error.
fn pane_id(ctx: &Ctx, arg: &str) -> Result<i64> {
    let arg = arg.trim();
    if arg.is_empty() {
        return Ok(ctx.workspace.devctl_focused_pane());
    }
    let id: i64 = arg.parse().map_err(|_| anyhow!("expected a pane id (`panes` lists them), got {arg:?}"))?;
    if ctx.workspace.pane_terminal(id).is_none() {
        return Err(anyhow!("no pane {id} in this window (`panes` lists them)"));
    }
    Ok(id)
}

pub(super) fn note(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.set_first_note(arg.to_string(), ctx.window, ctx.cx);
    Ok(String::new())
}

pub(super) fn task(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.add_task(arg.to_string(), ctx.cx);
    Ok(String::new())
}

/// `front`: this window to the front with the keys, even in background mode, which otherwise keeps
/// the instance behind whatever the person is using.
pub(super) fn front(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.cx.activate(true);
    ctx.window.activate_window();
    Ok(String::new())
}

/// `profile <id>`: switch Wormhole to that profile, after this batch (the windows are updating now).
pub(super) fn profile(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let id = wormhole_core::profile::ProfileId::parse(arg.trim()).map_err(|err| anyhow!("{err}"))?;
    ctx.cx.defer(move |cx| {
        if let Err(err) = crate::workspace::AppState::switch_profile(&id, cx) {
            tracing::warn!("devctl profile: {err}");
        }
    });
    Ok(String::new())
}

/// `profile-new <name> [git-email]`: make a profile with Wormhole's defaults (with a git identity
/// of that name and email, when an email is given) and print its id.
pub(super) fn profile_new(ctx: &mut Ctx, arg: &str) -> Result<String> {
    use wormhole_core::profile::{NewProfile, StartFrom};
    let state = crate::workspace::app_mut(ctx.cx);
    let root = state.root.clone();
    let (name, email) = match arg.trim().rsplit_once(' ') {
        Some((name, email)) if email.contains('@') => (name.trim(), Some(email)),
        _ => (arg.trim(), None),
    };
    let new = NewProfile { name: name.to_string(), color: None, avatar: None, from: StartFrom::Defaults };
    let id = state.profiles.create(&root, new).map_err(|err| anyhow!("{err}"))?;
    if let Some(email) = email {
        let (identity_name, email) = (name.to_string(), email.to_string());
        state
            .profiles
            .edit(&id, |p| p.git = wormhole_core::profile::GitIdentity { name: identity_name, email })
            .and_then(|()| state.profiles.save(&root))
            .map_err(|err| anyhow!("{err}"))?;
    }
    Ok(id.to_string())
}

/// `profiles`: one a line, `*` on the shown one.
pub(super) fn profiles(ctx: &mut Ctx, _: &str) -> Result<String> {
    let state = crate::workspace::app(ctx.cx);
    Ok(state
        .profiles
        .profiles()
        .iter()
        .map(|p| {
            let mark = if &p.id == state.profiles.active_id() { '*' } else { ' ' };
            format!("{mark} {} {:?} {:?}\n", p.id, p.name, p.color)
        })
        .collect())
}

pub(super) fn quit(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.quit(ctx.cx);
    Ok(String::new())
}

pub(super) fn resize(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut dims = arg.split_whitespace().filter_map(|n| n.parse::<f32>().ok());
    let (Some(w), Some(h)) = (dims.next(), dims.next()) else {
        return Err(anyhow!("expected <w> <h>"));
    };
    ctx.window.resize(size(px(w), px(h)));
    Ok(String::new())
}

pub(super) fn toggle_sidebar(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.toggle_sidebar(ctx.cx);
    Ok(String::new())
}

/// Show or hide the right panel.
pub(super) fn toggle_panel(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.toggle_notes(ctx.window, ctx.cx);
    Ok(String::new())
}

pub(super) fn stash(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.stash_prompt(ctx.window, ctx.cx);
    Ok(String::new())
}

/// Send the focused note to the terminal, pressing Enter after it when `submit`.
pub(super) fn send(ctx: &mut Ctx, _: &str, submit: bool) -> Result<String> {
    ctx.workspace.send_focused_note(submit, ctx.window, ctx.cx);
    Ok(String::new())
}

/// An agent's hook event for one of this window's terminals, as the hook socket delivers it.
pub(super) fn hook(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let hook = Hook::parse(arg)?;
    ctx.workspace.devctl_hook(hook.tab, hook.agent, &hook.event, hook.message, ctx.window, ctx.cx)?;
    Ok(String::new())
}

/// `[--agent <id>] <tab> <event> [message]`.
#[derive(Debug, PartialEq)]
struct Hook {
    agent: AgentId,
    tab: i64,
    event: String,
    message: Option<String>,
}

impl Hook {
    fn parse(arg: &str) -> Result<Self> {
        let usage = || anyhow!("expected [--agent <id>] <tab> <event> [message]");
        let mut rest = arg.trim();
        // Claude's, like a hook that names no agent (see `ipc::Message`).
        let mut agent = AgentId::Claude;
        if let Some(after) = rest.strip_prefix("--agent ") {
            let (id, tail) = after.trim_start().split_once(' ').ok_or_else(usage)?;
            agent = serde_json::from_value(serde_json::Value::String(id.to_string()))
                .map_err(|_| anyhow!("unknown agent {id:?}"))?;
            rest = tail.trim_start();
        }
        let mut parts = rest.splitn(3, ' ');
        let tab = parts.next().and_then(|tab| tab.parse().ok()).ok_or_else(usage)?;
        let event = parts.next().filter(|event| !event.is_empty()).ok_or_else(usage)?.to_string();
        let message = parts.next().map(str::trim).filter(|m| !m.is_empty()).map(str::to_string);
        Ok(Self { agent, tab, event, message })
    }
}

/// `onboarding <step>`: the intro window at that step, or `close`.
pub(super) fn onboarding(ctx: &mut Ctx, arg: &str) -> Result<String> {
    use wormhole_core::onboarding::Step;
    let step = match arg.trim() {
        "welcome" | "" => Step::Welcome,
        "folder" => Step::Folder,
        "look" => Step::Look,
        "agents" => Step::Agents,
        "primer" => Step::Primer,
        "close" => {
            ctx.workspace.close_onboarding(ctx.window, ctx.cx);
            return Ok(String::new());
        }
        other => return Err(anyhow!("expected welcome|folder|look|agents|primer|close, got {other:?}")),
    };
    ctx.workspace.open_onboarding(step, ctx.window, ctx.cx);
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use wormhole_core::agents::AgentId;

    use super::{Hook, shell_arg};

    #[test]
    fn a_shell_argument_is_a_key_and_maybe_a_folder() {
        assert_eq!(shell_arg("fish").expect("parses"), ("fish".into(), None));
        assert_eq!(shell_arg(" fish  ~/src ").expect("parses"), ("fish".into(), Some("~/src".into())));
        assert_eq!(
            shell_arg("- /tmp").expect("parses"),
            (String::new(), Some("/tmp".into())),
            "- is the login shell"
        );
        assert_eq!(shell_arg("/bin/bash").expect("parses").0, "/bin/bash");
        assert!(shell_arg("").is_err());
    }

    #[test]
    fn a_hook_names_a_tab_and_an_event_and_may_carry_a_message() {
        let hook = Hook::parse("2 Notification Allow Bash?").expect("parses");
        assert_eq!(
            hook,
            Hook {
                agent: AgentId::Claude,
                tab: 2,
                event: "Notification".into(),
                message: Some("Allow Bash?".into())
            }
        );
        assert_eq!(Hook::parse(" 7 Stop ").expect("parses").message, None);
    }

    #[test]
    fn the_agent_is_claude_unless_named() {
        let hook = Hook::parse("--agent codex 3 PermissionRequest").expect("parses");
        assert_eq!((hook.agent, hook.tab, hook.event.as_str()), (AgentId::Codex, 3, "PermissionRequest"));
        assert_eq!(Hook::parse("--agent opencode 1 session.idle").expect("parses").agent, AgentId::OpenCode);
        assert!(Hook::parse("--agent nobody 1 Stop").is_err());
    }

    #[test]
    fn a_hook_without_a_tab_or_an_event_is_refused() {
        for bad in ["", "Stop", "x Stop", "1", "--agent claude", "--agent claude 1"] {
            assert!(Hook::parse(bad).is_err(), "{bad:?}");
        }
    }
}
