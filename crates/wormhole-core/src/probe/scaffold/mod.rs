//! `Wormhole probe new`: the files of a new Probe, each kind with a complete working example
//! (`repo-docs/docs/probes.md`, section 5).
//!
//! The files are made in memory first and validated like any Probe, so a scaffold that would not
//! validate (an id that is a built-in agent's name, say) is refused instead of written. They are written
//! through `storage::files` into a folder that did not exist: an existing one is never touched.
//!
//! - `plan`: the files of each kind, from the texts in `templates/`.
//!
//! Must not use anything above `validate`; it knows no settings and no data folder.

mod plan;

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::read::ProbeFolder;
use super::validate::Finding;
use crate::storage::files::{self, LinkPolicy};

/// What a new Probe contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Text rules: a link, a shortened email, a hidden key.
    Rules,
    /// A rule that hands a commit id to a palette command, and a script.
    Command,
    /// A status bar item fed by a command.
    Status,
    /// A coding agent to recognise.
    Agent,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Self::Rules, Self::Command, Self::Status, Self::Agent];

    /// The word `--kind` takes.
    pub fn word(self) -> &'static str {
        match self {
            Self::Rules => "rules",
            Self::Command => "command",
            Self::Status => "status",
            Self::Agent => "agent",
        }
    }

    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.word() == word)
    }
}

/// Why a Probe was not made.
#[derive(Debug)]
pub enum ScaffoldError {
    /// Not an id (2 to 32 characters of `a-z`, `0-9` and `-`).
    BadId(String),
    /// The folder to make it in is not there.
    NoParent(PathBuf),
    /// Something is already at the path, a folder or not: never replaced.
    Exists(PathBuf),
    /// The example would not validate with this id.
    Invalid(Vec<Finding>),
    /// A template of this build is not a manifest. The tests read every one, so this is a bug.
    Template(String),
    Io {
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for ScaffoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadId(id) => write!(
                f,
                "{id:?} is not a Probe id: use 2 to 32 characters from a-z, 0-9 and `-`, \
                 starting and ending with a letter or a digit"
            ),
            Self::NoParent(path) => write!(f, "{} is not a folder", path.display()),
            Self::Exists(path) => write!(f, "{} already exists; nothing was changed", path.display()),
            Self::Invalid(findings) => {
                f.write_str("a new Probe with this id would not validate: ")?;
                for (ix, finding) in findings.iter().enumerate() {
                    if ix > 0 {
                        f.write_str("; ")?;
                    }
                    f.write_str(&finding.message)?;
                }
                Ok(())
            }
            Self::Template(why) => write!(f, "the example for this kind is broken: {why}"),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for ScaffoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// A Probe made on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    pub root: PathBuf,
    /// The files, relative to `root`, with `/` between parts.
    pub files: Vec<String>,
}

/// The Probe `id` of `kind` as `create` would write it, validated, without writing anything.
pub fn preview(id: &str, kind: Kind) -> Result<ProbeFolder, ScaffoldError> {
    plan::build(id, kind).map(|(_, folder)| folder)
}

/// Make `<parent>/<id>/` a Probe of `kind`. `parent` must exist; `<parent>/<id>` must not.
pub fn create(parent: &Path, id: &str, kind: Kind) -> Result<Created, ScaffoldError> {
    create_with(parent, id, kind, &mut write_file)
}

/// Writes one file of a Probe: its path, the canonical folder it must stay inside, its bytes and
/// whether it is a program.
type WriteFile<'a> = dyn FnMut(&Path, &Path, &[u8], bool) -> io::Result<()> + 'a;

fn write_file(path: &Path, root: &Path, bytes: &[u8], executable: bool) -> io::Result<()> {
    let policy = LinkPolicy::Within(root);
    if executable {
        files::write_atomic_executable(path, bytes, policy)
    } else {
        files::write_atomic(path, bytes, policy)
    }
}

/// [`create`] with the writing of each file in the caller's hands, so a failure half way can be tried.
fn create_with(
    parent: &Path,
    id: &str,
    kind: Kind,
    write: &mut WriteFile<'_>,
) -> Result<Created, ScaffoldError> {
    let (planned, _) = plan::build(id, kind)?;
    match fs::metadata(parent) {
        Ok(meta) if meta.is_dir() => {}
        _ => return Err(ScaffoldError::NoParent(parent.to_path_buf())),
    }
    let root = parent.join(id);
    if fs::symlink_metadata(&root).is_ok() {
        return Err(ScaffoldError::Exists(root));
    }
    // Exclusive: if something made it since the check, this fails instead of writing into it.
    fs::create_dir(&root).map_err(|source| match source.kind() {
        io::ErrorKind::AlreadyExists => ScaffoldError::Exists(root.clone()),
        _ => ScaffoldError::Io { path: root.clone(), source },
    })?;
    // Resolved once, now that the folder is ours: every write is held to this real path, so a link put
    // in the place of `<parent>` or of `<id>` afterwards cannot send the files elsewhere.
    let real = match fs::canonicalize(&root) {
        Ok(real) => real,
        Err(source) => {
            let _ = fs::remove_dir(&root);
            return Err(ScaffoldError::Io { path: root, source });
        }
    };
    let mut made = Made::default();
    for file in &planned {
        let path = real.join(file.path);
        made.folders_for(&real, &path);
        let result = write(&path, &real, file.text.as_bytes(), file.executable);
        if let Err(source) = result {
            made.undo(&real);
            return Err(ScaffoldError::Io { path: root.join(file.path), source });
        }
        made.files.push(path);
    }
    Ok(Created { root, files: planned.iter().map(|file| file.path.to_string()).collect() })
}

/// What a `create` has made so far, to take back exactly that and nothing else.
#[derive(Default)]
struct Made {
    files: Vec<PathBuf>,
    /// Folders below the Probe's, outermost first.
    folders: Vec<PathBuf>,
}

impl Made {
    /// Note the folders between `root` and `file` that are not there yet, which writing it creates.
    fn folders_for(&mut self, root: &Path, file: &Path) {
        let mut missing: Vec<PathBuf> = file
            .ancestors()
            .skip(1)
            .take_while(|dir| *dir != root && dir.starts_with(root))
            .filter(|dir| !dir.exists())
            .map(Path::to_path_buf)
            .collect();
        missing.reverse();
        self.folders.extend(missing);
    }

    /// Remove what was made: each file, then each folder, deepest first, then the Probe's own folder.
    /// Never a recursive removal, so a folder that holds anything else is left as it is.
    fn undo(&self, root: &Path) {
        for file in self.files.iter().rev() {
            let _ = fs::remove_file(file);
        }
        for dir in self.folders.iter().rev() {
            let _ = fs::remove_dir(dir);
        }
        let _ = fs::remove_dir(root);
    }
}

#[cfg(test)]
mod tests;
