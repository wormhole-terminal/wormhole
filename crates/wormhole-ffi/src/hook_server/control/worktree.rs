//! `new-terminal --worktree <branch>`: the GPUI app opens its dialog for it (`features/git/worktree/ask.rs`);
//! this app puts its own question up (`HookEvent::WorktreeAsked`), after core's plan says the branch is one git
//! takes. Whether the project trusts a worktree is the window's to decide when the question comes up (git knows
//! nothing of trust), and so is making it: the reply says only that the question was asked. An untrusted project is
//! refused here, as the GPUI app refuses it, and a sheet already up is answered Failed.

use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError};

use wormhole_core::config::features::FeatureFlag;
use wormhole_core::git::worktree;
use wormhole_core::git::{find_root, worktree_of};
use wormhole_core::ipc::control::{Asked, Error, ErrorCode, Reply};

use crate::hook_server::core::Core;
use crate::hook_server::events::HookEvent;
use crate::workspace::Shared;

/// Where the sheet would put the worktree: the same rule as the service the sheet plans with.
fn base(core: &Core) -> Option<PathBuf> {
    crate::git::worktree_base(Some(core.settings().worktree_base.clone()))
}

/// `window` is the window the request reached: its profile judges trust, its sheet is the one asked, and the question
/// comes up there.
pub(super) fn ask(
    core: &Arc<Core>,
    window: &Shared,
    folder: Option<String>,
    branch: &str,
    command: Option<String>,
) -> Reply {
    // The command goes with the question (`HookEvent::WorktreeAsked`): the window types it into the terminal that opens
    // once the person confirms, as the GPUI app's dialog does (`features/git/worktree/ask.rs`).
    let start = folder
        .map(PathBuf::from)
        .or_else(|| window.peek_model(|model| model.selected().and_then(|id| model.facts(id)?.cwd.clone())));
    let Some(start) = start else {
        return Error::new(ErrorCode::NotFound, "no folder to make a worktree of").into();
    };
    let Some(root) = find_root(&start) else {
        return Error::new(ErrorCode::NotFound, format!("{} is not in a git repository", start.display()))
            .into();
    };
    let repo: PathBuf = worktree_of(&root).unwrap_or(root);
    // Trust is asked of the repository, never of the terminal's folder; with the project folders off nothing is.
    let seat = core.seat_of_window(window);
    if seat.settings().feature_enabled(FeatureFlag::ProjectFolders)
        && let Some(why) = crate::files_panel::worktree_refusal_in(seat.profile.dir(), &repo)
    {
        return Error::new(ErrorCode::Refused, why).into();
    }
    let plan = match worktree::plan(&repo, branch, base(core).as_deref()) {
        Ok(plan) => plan,
        Err(why) => return Error::new(ErrorCode::BadRequest, why).into(),
    };
    let id = window.window_id();
    if !core.worktree_question.lock().unwrap_or_else(PoisonError::into_inner).insert(id) {
        return Error::new(ErrorCode::Failed, "a worktree dialog is already up in that window").into();
    }
    let command = command.map(|command| command.trim().to_string()).filter(|command| !command.is_empty());
    core.ui.push(HookEvent::WorktreeAsked {
        window: id,
        folder: shown(&repo),
        branch: plan.branch.clone(),
        command,
    });
    tracing::info!(branch = plan.branch, "control: the worktree question was asked");
    Reply::ok(Asked {
        asked: true,
        window: id,
        branch: plan.branch,
        folder: plan.folder.to_string_lossy().into_owned(),
    })
}

fn shown(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
