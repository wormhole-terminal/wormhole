//! A worktree per terminal: planning, making and removing the ones Wormhole made, and remembering them
//! across launches (core's `git::worktree`, which writes through `GitWrite`: `worktree add` and
//! `worktree remove`, hooks off, never forced). Nothing here runs by itself: each write is one explicit call
//! the person confirmed in a sheet. Whether the *project* trusts a worktree is the caller's to decide before
//! `create` (it needs the project feature's trust store, which core keeps apart from git).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use wormhole_core::git::worktree::{self, Worktree, Worktrees};
use wormhole_core::git::{find_root, worktree_of};
use wormhole_core::storage::paths::ProfilePaths;
use wormhole_core::storage::store::Store;

use super::error::GitBackendError;

/// A worktree Wormhole made: of which repository (its main working tree), on which branch, where.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitWorktree {
    pub repo: String,
    pub branch: String,
    pub folder: String,
    /// What its terminals get in their environment (`WORMHOLE_PORT`, `WORMHOLE_PORTS`), when it has ports.
    pub env: Vec<GitEnvVar>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitEnvVar {
    pub name: String,
    pub value: String,
}

impl GitWorktree {
    fn of(made: &Worktree) -> Self {
        Self {
            repo: made.repo.to_string_lossy().into_owned(),
            branch: made.branch.clone(),
            folder: made.folder.to_string_lossy().into_owned(),
            env: made.env().into_iter().map(|(name, value)| GitEnvVar { name: name.into(), value }).collect(),
        }
    }
}

/// What a typed branch would make.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum GitWorktreePlan {
    /// The branch as git accepts it, and the folder the worktree would go in.
    Ready { branch: String, folder: String },
    /// Why not, in the sheet's words.
    Refused { reason: String },
}

struct State {
    /// `worktree_base` of the settings: where worktrees go instead of beside the repository.
    base: Option<PathBuf>,
    clone_build_folders: bool,
    /// The folder of the window's own profile, whose database remembers the worktrees (and so the ports); `None`
    /// until a window says (the data folder's default).
    profile: Option<PathBuf>,
}

#[derive(uniffi::Object)]
pub struct WorktreeService {
    state: Mutex<State>,
}

#[uniffi::export]
impl WorktreeService {
    /// `base` and `clone_build_folders` are the settings `worktree_base` and `worktree_clone_build_folders`.
    #[uniffi::constructor]
    pub fn new(base: Option<String>, clone_build_folders: bool) -> Result<Arc<Self>, GitBackendError> {
        crate::isolation::require()?;
        let state = State { base: base_of(base), clone_build_folders, profile: None };
        Ok(Arc::new(Self { state: Mutex::new(state) }))
    }

    /// The settings changed.
    pub fn set_options(&self, base: Option<String>, clone_build_folders: bool) {
        let mut state = self.state();
        state.base = base_of(base);
        state.clone_build_folders = clone_build_folders;
    }

    /// The window moved to profile `profile` (or opened in it): the worktrees are that profile's from here on, as the GPUI
    /// app keeps them per profile. False (nothing changes) for a profile that is not in `profiles.json`.
    pub fn follow_profile(&self, profile: String) -> bool {
        let Some(dir) = crate::profile::dir_of(&profile) else { return false };
        let mut state = self.state();
        if state.profile.as_deref() != Some(dir.as_path()) {
            state.profile = Some(dir);
        }
        true
    }

    /// The repository a worktree of `folder` is of: its main working tree (a terminal in a linked worktree makes
    /// siblings of the same repository), and the folder whose project decides trust. `None` outside a repository.
    pub fn repository(&self, folder: String) -> Option<String> {
        repo_of(&folder).ok().map(|repo| repo.to_string_lossy().into_owned())
    }

    /// Why a worktree can't be made of `folder`: it is in no repository. `None` when it can.
    pub fn problem(&self, folder: String) -> Option<String> {
        repo_of(&folder).err()
    }

    /// The branch name as git's rules read it, and where the folder goes, without writing anything.
    pub fn plan(&self, branch: String, folder: String) -> GitWorktreePlan {
        let base = self.state().base.clone();
        let planned = repo_of(&folder).and_then(|repo| worktree::plan(&repo, &branch, base.as_deref()));
        match planned {
            Ok(plan) => GitWorktreePlan::Ready {
                branch: plan.branch,
                folder: plan.folder.to_string_lossy().into_owned(),
            },
            Err(reason) => GitWorktreePlan::Refused { reason },
        }
    }

    /// Make the worktree `plan` says, on a new branch from HEAD or on the one the repository has; clone the
    /// build folders when the settings ask; give it ports; remember it. Blocking: up to git's checkout.
    pub fn create(&self, branch: String, folder: String) -> Result<GitWorktree, GitBackendError> {
        let (base, clone) = {
            let state = self.state();
            (state.base.clone(), state.clone_build_folders)
        };
        let repo = repo_of(&folder).map_err(GitBackendError::failed)?;
        let mut made = worktree::plan(&repo, &branch, base.as_deref()).map_err(GitBackendError::failed)?;
        worktree::add(&made).map_err(|err| GitBackendError::failed(format!("{err:#}")))?;
        // Failures are logged inside: the worktree exists and works without the clones.
        if clone {
            made.cloned = worktree::clone_build_folders(&made.repo, &made.folder);
        }
        // Read, picked and written under the one lock, from what the profile's database holds now: another window of the
        // profile may have made one since, and its record and its ports stay.
        let profile = self.state().profile.clone();
        made.port = update(profile.as_deref(), |all| {
            let port = all.free_ports(worktree::port_free);
            let mut recorded = made.clone();
            recorded.port = port;
            all.record(recorded);
            (port, true)
        });
        tracing::info!(branch = made.branch, folder = %made.folder.display(), port = ?made.port, "worktree made");
        Ok(GitWorktree::of(&made))
    }

    /// The worktree Wormhole made that `folder` is in (or under), if any. Worktrees whose folder is gone
    /// are forgotten first.
    pub fn made_at(&self, folder: String) -> Option<GitWorktree> {
        let profile = self.state().profile.clone();
        update(profile.as_deref(), |all| {
            let pruned = all.prune();
            (all.find(Path::new(&folder)).map(GitWorktree::of), pruned)
        })
    }

    /// Every worktree Wormhole made and that still exists.
    pub fn list(&self) -> Vec<GitWorktree> {
        let profile = self.state().profile.clone();
        update(profile.as_deref(), |all| {
            (all.iter().filter(|w| w.folder.exists()).map(GitWorktree::of).collect(), false)
        })
    }

    /// Whether the worktree at `folder` has changed or untracked files, which keep it from being removed.
    pub fn is_dirty(&self, folder: String) -> Result<bool, GitBackendError> {
        worktree::is_dirty(Path::new(&folder)).map_err(|err| GitBackendError::of(&err))
    }

    /// Remove the folder of a worktree Wormhole made (its branch stays). Refused for one it did not make,
    /// and by git while the folder has changes: never forced.
    pub fn remove(&self, folder: String) -> Result<(), GitBackendError> {
        let profile = self.state().profile.clone();
        let recorded = update(profile.as_deref(), |all| {
            (all.find(Path::new(&folder)).filter(|w| w.folder == Path::new(&folder)).cloned(), false)
        });
        let Some(made) = recorded else {
            return Err(GitBackendError::failed(
                "Wormhole didn't make a worktree there, so it won't remove it",
            ));
        };
        worktree::remove(&made).map_err(|err| GitBackendError::failed(format!("{err:#}")))?;
        update(profile.as_deref(), |all| ((), all.forget(&made.folder)));
        Ok(())
    }
}

impl WorktreeService {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// One lock for every window's read-modify-write of the stored worktrees (the windows of a profile share its database, and
/// no window keeps a copy that could be written back stale).
static STORE_LOCK: Mutex<()> = Mutex::new(());

/// Runs `edit` on the worktrees the profile's database holds now and writes them back when it says it changed them.
fn update<R>(profile: Option<&Path>, edit: impl FnOnce(&mut Worktrees) -> (R, bool)) -> R {
    let _held = STORE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let store = open_store(profile);
    let stored = store.as_ref().ok().and_then(|store| store.meta(worktree::STORE_KEY).ok().flatten());
    let mut all = stored.map(|json| Worktrees::from_json(&json)).unwrap_or_default();
    let (result, changed) = edit(&mut all);
    if changed {
        match store {
            Ok(store) => {
                if let Err(err) = store.set_meta(worktree::STORE_KEY, &all.to_json()) {
                    tracing::warn!("remembering the worktrees: {err:#}");
                }
            }
            Err(err) => tracing::warn!("remembering the worktrees: {err:#}"),
        }
    }
    result
}

/// What a terminal started in `cwd` gets from the worktree Wormhole made there: its own ports (GPUI's spawn hook,
/// `features/git/worktree/def.rs`). Read from the store as it is at the start, so every terminal of the app, local or
/// hosted, asks the same place; nothing for a folder that is no worktree of Wormhole's.
pub(crate) fn env_for(cwd: &Path, profile: &ProfilePaths) -> Vec<(String, String)> {
    if crate::isolation::require().is_err() {
        return Vec::new();
    }
    // The terminal's own profile's database, where that profile's windows remember the worktrees they made.
    let stored =
        Store::open_for(profile).ok().and_then(|store| store.meta(worktree::STORE_KEY).ok().flatten());
    let all = stored.map(|json| Worktrees::from_json(&json)).unwrap_or_default();
    all.find(cwd)
        .map(|made| made.env().into_iter().map(|(name, value)| (name.to_owned(), value)).collect())
        .unwrap_or_default()
}

/// The database of the profile in `dir`, the data folder's default when no window has said.
fn open_store(dir: Option<&Path>) -> anyhow::Result<Store> {
    match dir {
        Some(dir) => Store::open_at(dir),
        None => Store::open(),
    }
}

/// `worktree_base` as a folder (`~` is the caller's to expand); empty puts worktrees beside the repository.
/// `worktree_base` as a folder, `~` expanded; `None` (unset, blank) puts worktrees beside the repository.
pub(crate) fn base_of(base: Option<String>) -> Option<PathBuf> {
    let base = base?;
    let base = base.trim();
    if base.is_empty() {
        return None;
    }
    let home = wormhole_core::platform::paths::home();
    Some(match (base.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => home.join(rest),
        (None, Some(home)) if base == "~" => home,
        _ => PathBuf::from(base),
    })
}

/// The repository a worktree of `folder` is of: its main working tree, so a terminal in a worktree makes
/// its siblings of the same repository.
fn repo_of(folder: &str) -> Result<PathBuf, String> {
    let root = find_root(Path::new(folder)).ok_or_else(|| format!("{folder} is not in a git repository"))?;
    Ok(worktree_of(&root).unwrap_or(root))
}
