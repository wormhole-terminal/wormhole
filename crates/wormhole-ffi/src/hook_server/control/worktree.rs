//! `new-terminal --worktree <branch>`: the GPUI app opens its dialog for it (`features/git/worktree/ask.rs`);
//! this app puts its own question up (`HookEvent::WorktreeAsked`), after core's plan says the branch is one git
//! takes. Whether the project trusts a worktree is the window's to decide when the question comes up (git knows
//! nothing of trust), and so is making it: the reply says only that the question was asked.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use wormhole_core::git::worktree;
use wormhole_core::git::{find_root, worktree_of};
use wormhole_core::ipc::control::{Asked, Error, ErrorCode, Reply};

use crate::hook_server::core::Core;
use crate::hook_server::events::HookEvent;

/// Where the sheet would put the worktree: the same rule as the service the sheet plans with.
fn base(core: &Core) -> Option<PathBuf> {
    crate::git::worktree_base(Some(core.settings().worktree_base.clone()))
}

pub(super) fn ask(core: &Arc<Core>, folder: Option<String>, branch: &str, command: Option<String>) -> Reply {
    // The command goes with the question (`HookEvent::WorktreeAsked`): the window types it into the terminal that opens
    // once the person confirms, as the GPUI app's dialog does (`features/git/worktree/ask.rs`).
    let start = folder.map(PathBuf::from).or_else(|| {
        core.shared.peek_model(|model| model.selected().and_then(|id| model.facts(id)?.cwd.clone()))
    });
    let Some(start) = start else {
        return Error::new(ErrorCode::NotFound, "no folder to make a worktree of").into();
    };
    let Some(root) = find_root(&start) else {
        return Error::new(ErrorCode::NotFound, format!("{} is not in a git repository", start.display()))
            .into();
    };
    let repo: PathBuf = worktree_of(&root).unwrap_or(root);
    let plan = match worktree::plan(&repo, branch, base(core).as_deref()) {
        Ok(plan) => plan,
        Err(why) => return Error::new(ErrorCode::BadRequest, why).into(),
    };
    core.ui.push(HookEvent::WorktreeAsked { folder: shown(&repo), branch: plan.branch.clone(), command });
    tracing::info!(branch = plan.branch, "control: the worktree question was asked");
    Reply::ok(Asked {
        asked: true,
        window: core.window,
        branch: plan.branch,
        folder: plan.folder.to_string_lossy().into_owned(),
    })
}

fn shown(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
