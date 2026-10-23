//! The right panel: Changes, History, the diff, Notes and the Project tab.

use anyhow::{Result, anyhow};

use super::{Ctx, nth};
use crate::features::git::changes::devctl as changes_panel;
use crate::features::git::history::devctl as history_panel;
use crate::features::notes::devctl as notes;

pub(super) fn changes(ctx: &mut Ctx, _: &str) -> Result<String> {
    changes_panel::show(ctx.workspace, ctx.cx);
    Ok(String::new())
}

/// Open the n-th file's diff.
pub(super) fn diff(ctx: &mut Ctx, arg: &str) -> Result<String> {
    changes_panel::open_nth(ctx.workspace, nth(arg), ctx.cx);
    Ok(String::new())
}

/// Toggle Split / Unified.
pub(super) fn split(ctx: &mut Ctx, _: &str) -> Result<String> {
    changes_panel::toggle_split(ctx.workspace, ctx.cx);
    Ok(String::new())
}

pub(super) fn diff_scroll_x(ctx: &mut Ctx, arg: &str) -> Result<String> {
    changes_panel::scroll_x(ctx.workspace, -arg.trim().parse::<f32>().unwrap_or(0.0), ctx.cx);
    Ok(String::new())
}

/// The History tab; on `branch` when one is named (HEAD goes back to the current branch). A branch
/// that doesn't exist shows git's error, as one that was deleted does.
pub(super) fn history(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let branch = arg.trim();
    let branch = (!branch.is_empty()).then(|| (branch != "HEAD").then(|| branch.to_string()));
    history_panel::show(ctx.workspace, branch, ctx.cx);
    Ok(String::new())
}

pub(super) fn picker(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let (which, query) = arg.trim().split_once(' ').unwrap_or((arg.trim(), ""));
    match which {
        "history" => history_panel::picker(ctx.workspace, query, ctx.window, ctx.cx),
        _ => changes_panel::picker(ctx.workspace, query, ctx.window, ctx.cx),
    }
    Ok(String::new())
}

pub(super) fn commit(ctx: &mut Ctx, arg: &str) -> Result<String> {
    history_panel::toggle_commit(ctx.workspace, nth(arg), ctx.cx);
    Ok(String::new())
}

pub(super) fn commit_file(ctx: &mut Ctx, arg: &str) -> Result<String> {
    history_panel::commit_file(ctx.workspace, nth(arg), ctx.cx);
    Ok(String::new())
}

/// Compare with a branch (none, or HEAD, to stop comparing).
pub(super) fn compare(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let branch = (!arg.trim().is_empty() && arg.trim() != "HEAD").then(|| arg.trim().to_string());
    changes_panel::compare(ctx.workspace, branch, ctx.cx);
    Ok(String::new())
}

pub(super) fn project(ctx: &mut Ctx, _: &str) -> Result<String> {
    if !project_folders_on(ctx) {
        return Ok("project folders feature is off (feature_flags.project_folders)".into());
    }
    crate::features::project::devctl::show_tab(ctx.workspace, ctx.cx);
    Ok(String::new())
}

/// `project-init`: what the Project tab's Create project folder button does, for the active
/// terminal's folder. It is made off the UI thread: look at `rows` or `title` a tick later.
pub(super) fn project_init(ctx: &mut Ctx, _: &str) -> Result<String> {
    if !project_folders_on(ctx) {
        return Ok("project folders feature is off (feature_flags.project_folders)".into());
    }
    let folder = crate::features::project::devctl::init(ctx.workspace, ctx.cx).map_err(|e| anyhow!(e))?;
    Ok(format!("creating {}/.wormhole", folder.display()))
}

/// `project-trust`: trust the active terminal's project, as its trust button does.
pub(super) fn project_trust(ctx: &mut Ctx, _: &str) -> Result<String> {
    if !project_folders_on(ctx) {
        return Ok("project folders feature is off (feature_flags.project_folders)".into());
    }
    let root = crate::features::project::devctl::trust(ctx.workspace, ctx.cx).map_err(|e| anyhow!(e))?;
    Ok(format!("trusting {}", root.display()))
}

/// Whether the project-folder feature flag is on in this instance.
fn project_folders_on(ctx: &Ctx) -> bool {
    crate::settings::features(ctx.cx).has(wormhole_core::config::FeatureId::Project)
}

/// `trust-window`: the window listing everything the active terminal's project wants to run.
pub(super) fn trust_window(ctx: &mut Ctx, _: &str) -> Result<String> {
    crate::features::project::devctl::trust_window(ctx.workspace, ctx.cx)?;
    Ok(String::new())
}

/// `trust-shot [name]`: a screenshot of that window.
pub(super) fn trust_shot(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let name = super::file_name(arg, "trust")?;
    crate::features::project::devctl_trust_shot(name, ctx.dir, ctx.cx)
}

/// `panel-tab <changes|history|notes|usage|project>`: show that tab of the right panel.
pub(super) fn panel_tab(ctx: &mut Ctx, arg: &str) -> Result<String> {
    use wormhole_core::config::TabId;
    let arg = arg.trim();
    let Some(tab) = TabId::from_key(arg) else {
        return Err(anyhow!("expected changes, history, notes, usage or project, got {arg:?}"));
    };
    // A tab of a feature that is off can't be shown: say which switch to turn.
    if let Some(feature) = wormhole_core::config::FeatureId::of_tab(tab)
        && !crate::settings::features(ctx.cx).has(feature)
    {
        return Ok(format!("{} is off (feature_flags)", feature.info().title.to_lowercase()));
    }
    ctx.workspace.show_tab(tab, ctx.cx);
    Ok(String::new())
}

/// `task-seed <open> <done>`: that many next steps, the done ones spread among the open ones.
pub(super) fn task_seed(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut args = arg.split_whitespace().filter_map(|a| a.parse::<usize>().ok());
    let (open, done) = (args.next().unwrap_or(38), args.next().unwrap_or(12));
    notes::seed_tasks(ctx.workspace, open, done, ctx.cx);
    Ok(String::new())
}

/// `task-done <n>`: tick (or untick) the n-th next step.
pub(super) fn task_done(ctx: &mut Ctx, arg: &str) -> Result<String> {
    notes::toggle_task(ctx.workspace, nth(arg), ctx.cx);
    Ok(String::new())
}

/// `note-add <text>`: a note at the end of the list, open.
pub(super) fn note_add(ctx: &mut Ctx, arg: &str) -> Result<String> {
    notes::add_note(ctx.workspace, arg.to_string(), ctx.window, ctx.cx);
    Ok(String::new())
}

/// `note-toggle <n>`: fold or unfold the n-th note.
pub(super) fn note_toggle(ctx: &mut Ctx, arg: &str) -> Result<String> {
    notes::toggle_note(ctx.workspace, nth(arg), ctx.cx);
    Ok(String::new())
}

/// Fill the session with `<count>` notes of `<lines>` lines.
pub(super) fn note_seed(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut args = arg.split_whitespace().filter_map(|a| a.parse::<usize>().ok());
    let (count, lines) = (args.next().unwrap_or(50), args.next().unwrap_or(12));
    notes::seed_notes(ctx.workspace, count, lines, ctx.window, ctx.cx);
    Ok(String::new())
}

pub(super) fn panel_scroll(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_panel_scroll(arg.trim().parse().unwrap_or(0.0), ctx.cx);
    Ok(String::new())
}
