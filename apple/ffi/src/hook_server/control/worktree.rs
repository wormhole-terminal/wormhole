//! `new-terminal --worktree <branch>`: the GPUI app opens its dialog for it (`features/git/worktree/ask.rs`);
//! this app puts its own question up (`HookEvent::WorktreeAsked`), after core's plan says the branch is one git
//! takes. Whether the project trusts a worktree is the window's to decide when the question comes up (git knows
//! nothing of trust), and so is making it: the reply says only that the question was asked.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use wormhole_core::git::worktree;
use wormhole_core::git::{find_root, worktree_of};
use wormhole_core::ipc::control::{Asked, Error, ErrorCode, Reply};
use wormhole_core::platform::paths;

use crate::hook_server::core::Core;
use crate::hook_server::events::HookEvent;

/// `worktree_base` as a folder, `~` expanded; `None` puts worktrees beside the repository.
fn base(core: &Core) -> Option<PathBuf> {
    let settings = core.settings();
    let base = settings.worktree_base.trim();
    if base.is_empty() {
        return None;
    }
    let home = paths::home();
    Some(match (base.strip_prefix("~/"), &home) {
        (Some(rest), Some(home)) => home.join(rest),
        _ if base == "~" => home.clone().unwrap_or_else(|| PathBuf::from(base)),
        _ => PathBuf::from(base),
    })
}

pub(super) fn ask(core: &Arc<Core>, folder: Option<String>, branch: &str, command: Option<String>) -> Reply {
    if command.is_some() {
        // The terminal that opens after the question is answered is not this request's to type into.
        return Reply::error(
            ErrorCode::Unsupported,
            "a command can't be run in a terminal that waits for a worktree to be confirmed",
        );
    }
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
    core.ui.push(HookEvent::WorktreeAsked { folder: shown(&repo), branch: plan.branch.clone() });
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
