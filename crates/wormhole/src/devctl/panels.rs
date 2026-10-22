//! The right panel: Changes, History, the diff, Notes and the Project tab.

use anyhow::{Result, anyhow};

use super::{Ctx, nth};

pub(super) fn changes(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.show_changes(ctx.cx);
    Ok(String::new())
}

/// Open the n-th file's diff.
pub(super) fn diff(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.open_nth_diff(nth(arg), ctx.cx);
    Ok(String::new())
}

/// Toggle Split / Unified.
pub(super) fn split(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.toggle_split(ctx.cx);
    Ok(String::new())
}

pub(super) fn diff_scroll_x(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.scroll_diff_by(-arg.trim().parse::<f32>().unwrap_or(0.0), ctx.cx);
    Ok(String::new())
}

/// The History tab; on `branch` when one is named (HEAD goes back to the current branch). A branch
/// that doesn't exist shows git's error, as one that was deleted does.
pub(super) fn history(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_history(ctx.cx);
    let branch = arg.trim();
    if !branch.is_empty() {
        let branch = (branch != "HEAD").then(|| branch.to_string());
        ctx.workspace.devctl_history_branch(branch, ctx.cx);
    }
    Ok(String::new())
}

pub(super) fn picker(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_picker(arg.trim(), ctx.window, ctx.cx);
    Ok(String::new())
}

pub(super) fn commit(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_toggle_commit(nth(arg), ctx.cx);
    Ok(String::new())
}

pub(super) fn commit_file(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_commit_file(nth(arg), ctx.cx);
    Ok(String::new())
}

/// Compare with a branch (none, or HEAD, to stop comparing).
pub(super) fn compare(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let branch = (!arg.trim().is_empty() && arg.trim() != "HEAD").then(|| arg.trim().to_string());
    ctx.workspace.devctl_compare(branch, ctx.cx);
    Ok(String::new())
}

pub(super) fn project(ctx: &mut Ctx, _: &str) -> Result<String> {
    if !project_folders_on(ctx) {
        return Ok("project folders feature is off (feature_flags.project_folders)".into());
    }
    ctx.workspace.devctl_project_tab(ctx.cx);
    Ok(String::new())
}

/// `project-init`: what the Project tab's Create project folder button does, for the active
/// terminal's folder. It is made off the UI thread: look at `rows` or `title` a tick later.
pub(super) fn project_init(ctx: &mut Ctx, _: &str) -> Result<String> {
    if !project_folders_on(ctx) {
        return Ok("project folders feature is off (feature_flags.project_folders)".into());
    }
    let folder = ctx.workspace.devctl_project_init(ctx.cx).map_err(|e| anyhow!(e))?;
    Ok(format!("creating {}/.wormhole", folder.display()))
}

/// `project-trust`: trust the active terminal's project, as its trust button does.
pub(super) fn project_trust(ctx: &mut Ctx, _: &str) -> Result<String> {
    if !project_folders_on(ctx) {
        return Ok("project folders feature is off (feature_flags.project_folders)".into());
    }
    let root = ctx.workspace.devctl_project_trust(ctx.cx).map_err(|e| anyhow!(e))?;
    Ok(format!("trusting {}", root.display()))
}

/// Whether the project-folder feature flag is on in this instance.
fn project_folders_on(ctx: &Ctx) -> bool {
    crate::settings::features(ctx.cx).has(wormhole_core::config::FeatureId::Project)
}

/// `trust-window`: the window listing everything the active terminal's project wants to run.
pub(super) fn trust_window(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.devctl_trust_window(ctx.cx)?;
    Ok(String::new())
}

/// `trust-shot [name]`: a screenshot of that window.
pub(super) fn trust_shot(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let name = super::file_name(arg, "trust")?;
    crate::window::features::project::devctl_trust_shot(name, ctx.dir, ctx.cx)
}

/// `panel-tab <changes|history|notes|usage|project>`: show that tab of the right panel.
pub(super) fn panel_tab(ctx: &mut Ctx, arg: &str) -> Result<String> {
    use wormhole_core::config::TabId;
    let arg = arg.trim();
    let Some(tab) = TabId::from_key(arg) else {
        return Err(anyhow!("expected changes, history, notes, usage or project, got {arg:?}"));
    };
    if tab == TabId::Notes && !crate::window::features::notes::notes_on(ctx.cx) {
        return Ok("notes and tasks are off (feature_flags.notes_and_tasks)".into());
    }
    ctx.workspace.show_tab(tab, ctx.cx);
    Ok(String::new())
}

/// `task-seed <open> <done>`: that many next steps, the done ones spread among the open ones.
pub(super) fn task_seed(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut args = arg.split_whitespace().filter_map(|a| a.parse::<usize>().ok());
    let (open, done) = (args.next().unwrap_or(38), args.next().unwrap_or(12));
    ctx.workspace.devctl_seed_tasks(open, done, ctx.cx);
    Ok(String::new())
}

/// `task-done <n>`: tick (or untick) the n-th next step.
pub(super) fn task_done(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_toggle_task(nth(arg), ctx.cx);
    Ok(String::new())
}

/// `note-add <text>`: a note at the end of the list, open.
pub(super) fn note_add(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_add_note(arg.to_string(), ctx.window, ctx.cx);
    Ok(String::new())
}

/// `note-toggle <n>`: fold or unfold the n-th note.
pub(super) fn note_toggle(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_toggle_note(nth(arg), ctx.cx);
    Ok(String::new())
}

/// Fill the session with `<count>` notes of `<lines>` lines.
pub(super) fn note_seed(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut args = arg.split_whitespace().filter_map(|a| a.parse::<usize>().ok());
    let (count, lines) = (args.next().unwrap_or(50), args.next().unwrap_or(12));
    ctx.workspace.devctl_seed_notes(count, lines, ctx.window, ctx.cx);
    Ok(String::new())
}

pub(super) fn panel_scroll(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_panel_scroll(arg.trim().parse().unwrap_or(0.0), ctx.cx);
    Ok(String::new())
}
