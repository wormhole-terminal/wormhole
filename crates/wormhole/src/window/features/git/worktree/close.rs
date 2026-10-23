//! The question before closing a terminal in a worktree Wormhole made (keep the worktree, or remove it
//! too), for one terminal and for a group, and the removal.

use std::path::Path;
use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::git::worktree::{self, Worktree};

use super::ask::{shown, tell};
use crate::app_state::{AppState, app, app_mut};
use crate::window::facts::{SessionFacts, WindowFacts};
use crate::window::feature::{CloseAlso, CloseExtra, CloseOf};

/// Whether the status last read for `session` lists changes.
fn has_changes(session: &SessionFacts<'_>) -> bool {
    session.git.status.as_ref().is_some_and(|status| !status.files.is_empty())
}

/// The worktree Wormhole made that the folder `cwd` is in, if any.
fn made_at(cwd: &Path, cx: &App) -> Option<Worktree> {
    app(cx).worktrees.find(cwd).cloned()
}

/// What closing `sessions` adds to the question: which worktrees Wormhole made they are in, and what
/// Close and Close and Remove do with them. Nothing when none is.
pub(super) fn extra(facts: &WindowFacts<'_>, sessions: &[i64], of: CloseOf, cx: &App) -> Option<CloseExtra> {
    match of {
        CloseOf::Terminal => one(facts, *sessions.first()?, cx),
        CloseOf::Group => group(facts, sessions, cx),
    }
}

/// Closing a terminal in a worktree Wormhole made: Close keeps the worktree, Close and Remove Worktree
/// deletes its folder too (the branch stays); not offered while the folder has changes as last read, and
/// refused again by git itself if it has some by then.
fn one(facts: &WindowFacts<'_>, id: i64, cx: &App) -> Option<CloseExtra> {
    let session = facts.sessions().find(|session| session.id == id)?;
    let made = made_at(session.folder(cx)?, cx)?;
    let dirty = has_changes(&session);
    let note = format!(
        "Its worktree {} at {} was made by Wormhole. {}",
        made.branch,
        shown(&made.folder),
        if dirty {
            "It has uncommitted changes, so it stays."
        } else {
            "Close keeps it; Close and Remove Worktree deletes its folder (the branch stays)."
        }
    );
    let also = (!dirty).then(|| CloseAlso {
        label: "Close and Remove Worktree",
        then: Rc::new(move |cx: &mut App| remove_worktree(made.clone(), cx)),
    });
    Some(CloseExtra { note, also })
}

/// The worktrees Wormhole made that a group's terminals are in, for the question before closing the
/// group: each once, and whether it may be offered for removal (no changes as last read, and no
/// terminal outside the group still in it).
fn group(facts: &WindowFacts<'_>, members: &[i64], cx: &App) -> Option<CloseExtra> {
    let mut found: Vec<(Worktree, bool)> = Vec::new();
    for session in facts.sessions().filter(|session| members.contains(&session.id)) {
        let Some(cwd) = session.folder(cx) else { continue };
        let Some(made) = made_at(cwd, cx) else { continue };
        if found.iter().any(|(w, _)| w.folder == made.folder) {
            continue;
        }
        let shared = facts.sessions().any(|other| {
            !members.contains(&other.id) && other.folder(cx).is_some_and(|c| c.starts_with(&made.folder))
        });
        found.push((made, !has_changes(&session) && !shared));
    }
    if found.is_empty() {
        return None;
    }
    let names = found
        .iter()
        .map(|(w, _)| format!("{} at {}", w.branch, shown(&w.folder)))
        .collect::<Vec<_>>()
        .join(", ");
    let kept = found.iter().filter(|(_, removable)| !removable).count();
    let removable = found.len() - kept;
    let many = if found.len() == 1 { "worktree" } else { "worktrees" };
    let mut note = format!("Wormhole made the {many} {names}. ");
    match (removable, kept) {
        (0, _) => note.push_str("They have changes or other terminals in them, so they stay."),
        (_, 0) => note.push_str(
            "Close keeps them; Close and Remove Worktrees deletes their folders (the branches stay).",
        ),
        _ => note.push_str(
            "Close keeps them; Close and Remove Worktrees deletes the folders without changes or other \
             terminals (the branches stay).",
        ),
    }
    let removable: Vec<Worktree> =
        found.into_iter().filter(|(_, removable)| *removable).map(|(w, _)| w).collect();
    let also = (!removable.is_empty()).then(|| CloseAlso {
        label: "Close and Remove Worktrees",
        then: Rc::new(move |cx: &mut App| {
            for worktree in removable.clone() {
                remove_worktree(worktree, cx);
            }
        }),
    });
    Some(CloseExtra { note, also })
}

/// Remove `worktree` off the UI thread, once its terminal is closed; forgotten when it is gone, and
/// git's refusal (changes made in the meantime) shown in the front window.
pub(super) fn remove_worktree(worktree: Worktree, cx: &mut App) {
    cx.spawn(async move |cx| {
        let gone = worktree.clone();
        let result = cx.background_spawn(async move { worktree::remove(&gone) }).await;
        cx.update(|cx| match result {
            Ok(()) => {
                tracing::info!(folder = %worktree.folder.display(), "worktree removed");
                app_mut(cx).forget_worktree(&worktree.folder);
            }
            Err(err) => {
                tracing::warn!("removing a worktree: {err:#}");
                if let Some(front) = AppState::front_window(cx) {
                    let words = format!("{err:#}");
                    let _ = front.handle.update(cx, |_, window, cx| {
                        tell("Couldn't remove the worktree", words, window, cx);
                    });
                }
            }
        });
    })
    .detach();
}
