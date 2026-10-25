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
    let mut written = Vec::with_capacity(planned.len());
    for file in planned {
        let path = root.join(file.path);
        let policy = LinkPolicy::Within(&root);
        let result = if file.executable {
            files::write_atomic_executable(&path, file.text.as_bytes(), policy)
        } else {
            files::write_atomic(&path, file.text.as_bytes(), policy)
        };
        result.map_err(|source| ScaffoldError::Io { path, source })?;
        written.push(file.path.to_string());
    }
    Ok(Created { root, files: written })
}

#[cfg(test)]
mod tests;
