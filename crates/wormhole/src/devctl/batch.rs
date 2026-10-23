//! Reading the command file each window is given, running its lines one after another (a command may make
//! the rest of the batch wait for it) and writing what they printed.

use std::collections::{HashSet, VecDeque};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::storage::files::{self, LinkPolicy};

use super::commands::command;
use super::{Ctx, split_command};
use crate::window::Workspace;

/// Read each window's command file at the end of its tick, from the app's start on.
pub fn install(cx: &mut App) {
    cx.set_global(crate::window::devctl::TickHook(poll));
}

fn poll(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let Some(dir) = std::env::var_os("WORMHOLE_DEVCTL").map(PathBuf::from) else {
        return;
    };
    poll_dir(dir, workspace, window, cx);
}

/// [`poll`] with the control directory given (the view tests have no environment to set).
pub(crate) fn poll_dir(
    dir: PathBuf,
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    // A batch that waits (`wait-idle`) finishes before the next one is read, or the two would
    // interleave.
    if cx.try_global::<Waiting>().is_some_and(|waiting| waiting.0.contains(&cx.entity_id())) {
        return;
    }
    let number = workspace.window_number(cx);
    let suffix = if number == 1 { String::new() } else { number.to_string() };
    let out_path = dir.join(format!("out{suffix}.txt"));
    let Some(script) = take_script(&dir.join(format!("cmd{suffix}"))) else {
        return;
    };
    let batch = Batch {
        lines: script.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect(),
        out: String::new(),
        sync: Vec::new(),
        dir,
        out_path,
    };
    run(batch, workspace, window, cx);
}

/// One command file being worked through.
struct Batch {
    /// The lines still to run.
    lines: VecDeque<String>,
    /// What the commands printed so far.
    out: String,
    sync: Vec<String>,
    dir: PathBuf,
    out_path: PathBuf,
}

/// The workspaces (by entity) whose batch is waiting for a command to resolve.
#[derive(Default)]
struct Waiting(HashSet<EntityId>);

impl Global for Waiting {}

/// Run `batch` until it ends, or until a command makes the rest wait for it.
fn run(mut batch: Batch, workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    while let Some(line) = batch.lines.pop_front() {
        let (name, arg) = split_command(&line);
        let mut ctx = Ctx {
            workspace: &mut *workspace,
            window: &mut *window,
            cx: &mut *cx,
            dir: &batch.dir,
            out: &batch.out_path,
            wait: None,
            sync: Vec::new(),
        };
        let result = command(name).map(|run| run(&mut ctx, &arg));
        let (wait, mut sync) = (ctx.wait.take(), std::mem::take(&mut ctx.sync));
        batch.sync.append(&mut sync);
        match result {
            Some(Ok(text)) => batch.out.push_str(&text),
            Some(Err(err)) => {
                writeln!(batch.out, "{name} failed: {err:#}").ok();
            }
            None => {
                writeln!(batch.out, "unknown command: {name}").ok();
            }
        }
        if let Some(wait) = wait {
            resume_after(wait, batch, window, cx);
            return;
        }
    }
    finish(batch, workspace, cx);
}

/// Go on with `batch` once `wait` resolves, adding what it printed.
fn resume_after(wait: Task<String>, mut batch: Batch, window: &mut Window, cx: &mut Context<Workspace>) {
    let id = cx.entity_id();
    cx.default_global::<Waiting>().0.insert(id);
    cx.spawn_in(window, async move |this, cx| {
        batch.out.push_str(&wait.await);
        let resumed = this.update_in(cx, |workspace, window, cx| {
            cx.default_global::<Waiting>().0.remove(&id);
            run(batch, workspace, window, cx);
        });
        if resumed.is_err() {
            tracing::debug!("devctl: the window closed while its batch waited");
        }
    })
    .detach();
}

fn finish(batch: Batch, workspace: &Workspace, cx: &mut Context<Workspace>) {
    let _ = files::write_atomic(&batch.out_path, batch.out.as_bytes(), LinkPolicy::Follow);
    if !batch.sync.is_empty() {
        // Deferred, so the token comes after what commands of this batch append once the update
        // is over (`settings-shot`): a script that waits for it finds their results too.
        let (path, sync) = (batch.out_path, batch.sync);
        cx.defer(move |_| append(&path, &sync_lines(&sync)));
    }
    // A frame, as a terminal's output asks for one. What a command changed in the chrome has to have notified
    // the window itself: notifying it here would draw the chrome again and hide a command that did not.
    workspace.active_terminal().update(cx, |_, cx| cx.notify());
}

/// The lines `sync` writes for these tokens.
fn sync_lines(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|token| if token.is_empty() { "sync\n".into() } else { format!("sync {token}\n") })
        .collect()
}

/// Add `text` to the results file at `path`.
pub(super) fn append(path: &Path, text: &str) {
    use std::io::Write as _;
    let appended = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()));
    if let Err(err) = appended {
        tracing::warn!("devctl: could not append to {}: {err}", path.display());
    }
}

/// Read the command file and remove it. It is renamed first, so a line a writer appends
/// while this runs lands in a new file for the next tick; read-then-delete lost it.
fn take_script(path: &Path) -> Option<String> {
    let taken = path.with_extension("run");
    std::fs::rename(path, &taken).ok()?;
    let script = std::fs::read_to_string(&taken);
    let _ = std::fs::remove_file(&taken);
    script.ok()
}

#[cfg(test)]
mod tests {
    use super::{sync_lines, take_script};

    #[test]
    fn sync_writes_one_line_per_token_in_order() {
        assert_eq!(sync_lines(&["a1".into(), String::new(), "b 2".into()]), "sync a1\nsync\nsync b 2\n");
        assert_eq!(sync_lines(&[]), "");
    }

    #[test]
    fn taking_the_script_leaves_room_for_the_next_one() {
        let dir = std::env::temp_dir().join(format!("wormhole-devctl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let cmd = dir.join("cmd");
        assert_eq!(take_script(&cmd), None, "no file, no script");
        std::fs::write(&cmd, "state\nscreen\n").expect("write");
        assert_eq!(take_script(&cmd).as_deref(), Some("state\nscreen\n"));
        // The file is gone, and so is the one it was renamed to: a writer's next line starts
        // a new file.
        assert!(!cmd.exists() && !dir.join("cmd.run").exists());
        std::fs::write(&cmd, "state\n").expect("write again");
        assert_eq!(take_script(&cmd).as_deref(), Some("state\n"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
