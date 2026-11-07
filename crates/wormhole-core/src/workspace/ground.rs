//! The one place the model looks at the disk, behind a port: where a terminal's folder really is and
//! which repository it is in, for the groups that keep only the terminals in their folder (the
//! fence, `view::sidebar::group_folder`). A shell without a disk to ask (a preview, a test) hands
//! none and no group fences anything; the app's shell hands [`DiskGround`].

use std::fmt::Debug;
use std::path::{Path, PathBuf};

use crate::view::sidebar::group_folder::{Place, real_folder};

/// What the model asks of the file system.
pub trait Ground: Debug + Send + Sync {
    /// `path` as it really is: links followed, `..` resolved, and only when it is a folder.
    fn real_folder(&self, path: &Path) -> Option<PathBuf>;

    /// The repository `folder` is in: its root, or for a linked worktree the main working tree.
    fn repo_of(&self, folder: &Path) -> Option<PathBuf>;

    /// The main working tree of the repository `folder` is a linked worktree of, read from its `.git`
    /// file and not followed.
    fn worktree_of(&self, folder: &Path) -> Option<PathBuf>;
}

/// The real disk, through core's git readers (no git process).
#[derive(Clone, Copy, Debug, Default)]
pub struct DiskGround;

impl Ground for DiskGround {
    fn real_folder(&self, path: &Path) -> Option<PathBuf> {
        real_folder(path)
    }

    fn repo_of(&self, folder: &Path) -> Option<PathBuf> {
        let root = crate::git::find_root(folder)?;
        Some(crate::git::worktree_of(&root).unwrap_or(root))
    }

    fn worktree_of(&self, folder: &Path) -> Option<PathBuf> {
        let root = crate::git::find_root(folder).unwrap_or_else(|| folder.to_path_buf());
        crate::git::worktree_of(&root)
    }
}

/// Where a terminal in `cwd` is, as the groups' fences read it.
pub(super) fn place_of(ground: &dyn Ground, cwd: &Path) -> Option<Place> {
    Some(Place { folder: ground.real_folder(cwd)?, repo: None, linked_to: ground.worktree_of(cwd) })
}
