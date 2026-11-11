//! A worktree per terminal: planning, making and removing the ones Wormhole made, and remembering them
//! across launches (core's `git::worktree`, which writes through `GitWrite`: `worktree add` and
//! `worktree remove`, hooks off, never forced). Nothing here runs by itself: each write is one explicit call
//! the person confirmed in a sheet. Whether the *project* trusts a worktree is the caller's to decide before
//! `create` (it needs the project feature's trust store, which core keeps apart from git).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use wormhole_core::git::worktree::{self, Worktree, Worktrees};
use wormhole_core::git::{find_root, worktree_of};
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
    /// What the store holds under `worktree::STORE_KEY`, read once and written through.
    made: Option<Worktrees>,
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
        let state = State { base: base_of(base), clone_build_folders, made: None };
        Ok(Arc::new(Self { state: Mutex::new(state) }))
    }

    /// The settings changed.
    pub fn set_options(&self, base: Option<String>, clone_build_folders: bool) {
        let mut state = self.state();
        state.base = base_of(base);
        state.clone_build_folders = clone_build_folders;
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
        let mut state = self.state();
        let all = Self::load(&mut state);
        made.port = all.free_ports(worktree::port_free);
        all.record(made.clone());
        Self::save(&state);
        tracing::info!(branch = made.branch, folder = %made.folder.display(), port = ?made.port, "worktree made");
        Ok(GitWorktree::of(&made))
    }

    /// The worktree Wormhole made that `folder` is in (or under), if any. Worktrees whose folder is gone
    /// are forgotten first.
    pub fn made_at(&self, folder: String) -> Option<GitWorktree> {
        let mut state = self.state();
        let all = Self::load(&mut state);
        if all.prune() {
            Self::save(&state);
        }
        Self::load(&mut state).find(Path::new(&folder)).map(GitWorktree::of)
    }

    /// Every worktree Wormhole made and that still exists.
    pub fn list(&self) -> Vec<GitWorktree> {
        let mut state = self.state();
        Self::load(&mut state).iter().filter(|w| w.folder.exists()).map(GitWorktree::of).collect()
    }

    /// Whether the worktree at `folder` has changed or untracked files, which keep it from being removed.
    pub fn is_dirty(&self, folder: String) -> Result<bool, GitBackendError> {
        worktree::is_dirty(Path::new(&folder)).map_err(|err| GitBackendError::of(&err))
    }

    /// Remove the folder of a worktree Wormhole made (its branch stays). Refused for one it did not make,
    /// and by git while the folder has changes: never forced.
    pub fn remove(&self, folder: String) -> Result<(), GitBackendError> {
        let recorded = {
            let mut state = self.state();
            Self::load(&mut state)
                .find(Path::new(&folder))
                .filter(|w| w.folder == Path::new(&folder))
                .cloned()
        };
        let Some(made) = recorded else {
            return Err(GitBackendError::failed(
                "Wormhole didn't make a worktree there, so it won't remove it",
            ));
        };
        worktree::remove(&made).map_err(|err| GitBackendError::failed(format!("{err:#}")))?;
        let mut state = self.state();
        if Self::load(&mut state).forget(&made.folder) {
            Self::save(&state);
        }
        Ok(())
    }
}

impl WorktreeService {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn load(state: &mut State) -> &mut Worktrees {
        state.made.get_or_insert_with(|| {
            let stored = Store::open().ok().and_then(|store| store.meta(worktree::STORE_KEY).ok().flatten());
            stored.map(|json| Worktrees::from_json(&json)).unwrap_or_default()
        })
    }

    fn save(state: &State) {
        let Some(all) = &state.made else { return };
        match Store::open() {
            Ok(store) => {
                if let Err(err) = store.set_meta(worktree::STORE_KEY, &all.to_json()) {
                    tracing::warn!("remembering the worktrees: {err:#}");
                }
            }
            Err(err) => tracing::warn!("remembering the worktrees: {err:#}"),
        }
    }
}

/// What a terminal started in `cwd` gets from the worktree Wormhole made there: its own ports (GPUI's spawn hook,
/// `features/git/worktree/def.rs`). Read from the store as it is at the start, so every terminal of the app, local or
/// hosted, asks the same place; nothing for a folder that is no worktree of Wormhole's.
pub(crate) fn env_for(cwd: &Path) -> Vec<(String, String)> {
    if crate::isolation::require().is_err() {
        return Vec::new();
    }
    let stored = Store::open().ok().and_then(|store| store.meta(worktree::STORE_KEY).ok().flatten());
    let all = stored.map(|json| Worktrees::from_json(&json)).unwrap_or_default();
    all.find(cwd)
        .map(|made| made.env().into_iter().map(|(name, value)| (name.to_owned(), value)).collect())
        .unwrap_or_default()
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
