//! A repository's optional `.wormhole/` folder.
//!
//! Every loader returns `Ok(None)` when its file is absent, so each feature is optional. Whether a
//! feature is *used* is decided by the caller from [`ProjectPrefs`] (per-user toggles) and
//! [`TrustStore`] (nothing executable runs before the user approved the folder's current contents).
//!
//! Tier 2, beside `view`: it uses `storage` (capped reads that never leave the repository),
//! `keys` (an action's shortcut) and `theme` (the project's colour), never settings or the running
//! app. That is what lets `config`, one tier up, lay a trusted project's overrides over the
//! user's settings without a circle.

mod actions;
mod color;
mod create;
mod edit;
mod features;
mod notes;
mod prompts;
mod snapshot;
mod trust;

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::storage::fsutil::{self, LinkPolicy};

pub use actions::{Action, ActionKind, ChainStep, Target};
pub use color::{ProjectColor, ProjectColorChoice, TINT_PEAK, TINT_STOPS};
pub use create::init;
pub use features::{Feature, FeatureStanding, ProjectPrefs, trust_applies};
pub use notes::{
    NotesBase, NotesBaseStore, NotesDoc, NotesFile, SyncPlan, TaskItem, merge_notes, parse_notes_md,
    plan_sync, render_notes_md,
};
pub use prompts::Prompt;
pub use snapshot::ProjectSnapshot;
pub use trust::{
    TrustLine, TrustRequest, TrustStatus, TrustStore, TrustedProject, describe_trust_request, fingerprint,
};

/// Name of the project folder inside a repository.
pub const DIR_NAME: &str = ".wormhole";
/// Its name before the app took its product name (0.2 and before). Still read where there is no
/// `.wormhole/`, so repositories that have one keep working; a new one is always `.wormhole/`.
pub const LEGACY_DIR_NAME: &str = ".andromeda";
pub const CONFIG_FILE: &str = "config.json";
pub const THEME_FILE: &str = "theme.json";
pub const ACTIONS_FILE: &str = "actions.json";
pub const LAYOUT_FILE: &str = "layout.json";
pub const NOTES_FILE: &str = "notes.md";
pub const PROMPTS_DIR: &str = "prompts";

// Size limits for files read from a `.wormhole/` folder. Hand-written project files are a few
// kilobytes; the limits only stop a committed (or linked) huge file from stalling the thread that
// loads it. Exceeding one is a load error naming the file, never a silent truncation.

/// `config.json`, `theme.json`, `actions.json` and `layout.json`.
pub(crate) const MAX_JSON_BYTES: u64 = 1024 * 1024;
/// One prompt file (`prompts/*.md`, or the file a prompt action pastes).
pub(crate) const MAX_PROMPT_BYTES: u64 = 256 * 1024;
/// `notes.md`.
pub(crate) const MAX_NOTES_BYTES: u64 = 1024 * 1024;
/// Entries looked at in `prompts/`: more is a generated folder, not a prompt library.
pub(crate) const MAX_PROMPT_DIR_ENTRIES: usize = 1000;
/// The app's own state files (`trusted-projects.json`, `project-prefs.json`).
const MAX_STATE_BYTES: u64 = 16 * 1024 * 1024;

/// Error from loading a project file; always names the file, with a position when known.
#[derive(Debug)]
pub enum ProjectError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    /// Syntax or type error. `line`/`column` are 1-based, as reported by serde_json.
    Parse {
        path: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },
    /// Well-formed but semantically invalid (duplicate ids, unknown references, ...).
    Invalid {
        path: PathBuf,
        message: String,
    },
}

impl ProjectError {
    pub fn path(&self) -> &Path {
        match self {
            Self::Io { path, .. } | Self::Parse { path, .. } | Self::Invalid { path, .. } => path,
        }
    }

    fn invalid(path: &Path, message: impl Into<String>) -> Self {
        Self::Invalid { path: path.to_path_buf(), message: message.into() }
    }

    fn io(path: &Path, source: io::Error) -> Self {
        Self::Io { path: path.to_path_buf(), source }
    }

    /// A failed read: a refusal (outside the project, too large, not a file) is invalid
    /// content, anything else an I/O error.
    fn read(path: &Path, source: io::Error) -> Self {
        match fsutil::refusal(&source) {
            Some(refusal) => Self::invalid(path, refusal.to_string()),
            None => Self::io(path, source),
        }
    }

    /// What went wrong, without the path.
    fn reason(&self) -> String {
        match self {
            Self::Io { source, .. } => source.to_string(),
            Self::Parse { message, .. } | Self::Invalid { message, .. } => message.clone(),
        }
    }

    fn from_json(path: &Path, err: serde_json::Error) -> Self {
        // Errors from `from_value` have no position (line 0); report them as plain invalid data.
        if err.line() == 0 {
            return Self::invalid(path, err.to_string());
        }
        // serde_json appends " at line X column Y" to its message; we carry those separately.
        let message = err.to_string();
        let message = message.split(" at line ").next().unwrap_or(&message).to_string();
        Self::Parse { path: path.to_path_buf(), line: err.line(), column: err.column(), message }
    }
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Parse { path, line, column, message } => {
                write!(f, "{}:{line}:{column}: {message}", path.display())
            }
            Self::Invalid { path, message } => write!(f, "{}: {message}", path.display()),
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Something in a project file that was set aside rather than refused: the file still loads
/// and trust is not held up, but not all of it is used, and the Project tab says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectWarning {
    /// `project_color` isn't a colour, so the title row has no tint from it.
    Color { why: String },
    /// `project_color_locked` isn't true or false, so the colour is not locked.
    ColorLock { why: String },
    /// An action's `key` can't be used (an app key, one macOS keeps, no ⌘, a misspelling), so
    /// the action has none; it still runs from the palette and the menu.
    ActionKey { action: String, key: String, why: String },
}

impl fmt::Display for ProjectWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Color { why } => write!(f, "{CONFIG_FILE}: {why}; no colour is used"),
            Self::ColorLock { why } => write!(f, "{CONFIG_FILE}: {why}; the colour is not locked"),
            Self::ActionKey { action, why, .. } => {
                write!(f, "{ACTIONS_FILE}, {action:?}: {why}; this action has no key")
            }
        }
    }
}

/// Find the project folder governing `start` (a directory, or a file whose parent is used).
///
/// Rule: walk up from `start` (symlinks resolved) and return the first ancestor containing a
/// `.wormhole/` directory (or an older `.andromeda/` one). The walk stops at the filesystem root, and also at the first directory
/// containing `.git` (file or directory, so worktrees and submodules count): a repository's root
/// is the outermost directory that can configure it, so a `.wormhole/` above the repo (e.g. in
/// `$HOME`) never leaks into unrelated checkouts. A `.wormhole/` *in* the git root is found.
pub fn find_project(start: &Path) -> Option<ProjectFolder> {
    // Canonical paths make the root a stable key for prefs and trust, whatever path the user
    // opened (symlinks, `..`, trailing slashes).
    let start = start.canonicalize().ok()?;
    let start = if start.is_dir() { start } else { start.parent()?.to_path_buf() };
    for dir in start.ancestors() {
        if let Some(folder) = ProjectFolder::open(dir) {
            return Some(folder);
        }
        if dir.join(".git").exists() {
            return None;
        }
    }
    None
}

/// A discovered `.wormhole/` folder. `root` is the directory containing it (canonical when
/// found through [`find_project`]); relative paths in project files resolve against `root`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectFolder {
    pub root: PathBuf,
    pub dir: PathBuf,
}

/// `config.json`: everything except `env`, `shell`, `project_color` and `project_color_locked` is a
/// partial settings object, passed on unvalidated because the settings schema lives in the app.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectConfig {
    pub settings: serde_json::Map<String, serde_json::Value>,
    pub env: BTreeMap<String, String>,
    pub shell: Option<String>,
    /// `project_color`, when it is a colour. It only tints the folder's band and the title row, so
    /// it needs no trust.
    pub color: Option<ProjectColorChoice>,
    /// Why `project_color` was set aside, when it is there and isn't a colour.
    pub color_error: Option<String>,
    /// `project_color_locked` is `true`: Wormhole's colour menus leave the colour alone.
    pub color_locked: bool,
    /// Why `project_color_locked` was set aside, when it is there and isn't true or false.
    pub lock_error: Option<String>,
    /// `worktree.setup`: what a new worktree's terminal runs first (`npm install`, copying an
    /// `.env`), once, when Wormhole makes a worktree of this repository. It runs a command, so it
    /// needs trust like `shell` and `env`.
    pub worktree_setup: Option<String>,
}

/// `layout.json`: sessions to open when the project is opened.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    #[serde(default)]
    pub sessions: Vec<LayoutSession>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutSession {
    pub name: String,
    /// Command typed into the session's shell once it starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// Relative to the project root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub focus: bool,
}

/// The name of `root`'s project folder as it is on disk (`.wormhole`, or `.andromeda` in a
/// project from before the rename); `.wormhole` when there is none.
pub fn dir_name_at(root: &Path) -> String {
    ProjectFolder::open(root).map_or_else(|| DIR_NAME.to_string(), |folder| folder.dir_name())
}

impl ProjectFolder {
    /// The project folder of `root`, if `root/.wormhole/` (or, without it, the older
    /// `root/.andromeda/`) is a directory. Does not walk up.
    pub fn open(root: &Path) -> Option<Self> {
        [DIR_NAME, LEGACY_DIR_NAME]
            .into_iter()
            .map(|name| root.join(name))
            .find(|dir| dir.is_dir())
            .map(|dir| Self { root: root.to_path_buf(), dir })
    }

    /// The folder's name as it is on disk: `.wormhole`, or `.andromeda` in a project from before
    /// the rename.
    pub fn dir_name(&self) -> String {
        self.dir.file_name().map_or_else(|| DIR_NAME.to_string(), |name| name.to_string_lossy().into_owned())
    }

    pub fn path(&self, file: &str) -> PathBuf {
        self.dir.join(file)
    }

    /// Whether the files backing `feature` exist (for greying out toggles in the UI).
    pub fn has(&self, feature: Feature) -> bool {
        match feature {
            Feature::Prompts => self.path(PROMPTS_DIR).is_dir(),
            _ => self.path(feature.file_name()).is_file(),
        }
    }

    /// Resolve a project-relative `cwd` (from an action or layout session) to an absolute path.
    pub fn resolve_cwd(&self, cwd: Option<&str>) -> PathBuf {
        match cwd {
            Some(cwd) => self.root.join(cwd),
            None => self.root.clone(),
        }
    }

    /// Read one of this folder's files: every link resolved must stay inside the project root
    /// (a repository can commit `notes.md -> ~/.ssh/id_ed25519`), and at most `cap` bytes.
    fn read_text(&self, path: &Path, cap: u64) -> Result<Option<String>, ProjectError> {
        #[cfg(test)]
        tests::count_read();
        read_text(path, cap, LinkPolicy::Within(&self.root))
    }

    fn read_json<T: DeserializeOwned>(&self, path: &Path) -> Result<Option<T>, ProjectError> {
        let Some(text) = self.read_text(path, MAX_JSON_BYTES)? else { return Ok(None) };
        parse_json(path, &text).map(Some)
    }

    pub fn config(&self) -> Result<Option<ProjectConfig>, ProjectError> {
        let path = self.path(CONFIG_FILE);
        let Some(text) = self.read_text(&path, MAX_JSON_BYTES)? else { return Ok(None) };
        parse_config(&path, &text).map(Some)
    }

    /// Path of `theme.json` if present; the theme format belongs to the app.
    pub fn theme_path(&self) -> Option<PathBuf> {
        Some(self.path(THEME_FILE)).filter(|p| p.is_file())
    }

    pub fn theme_json(&self) -> Result<Option<serde_json::Value>, ProjectError> {
        self.read_json(&self.path(THEME_FILE))
    }

    /// Write `theme.json` atomically (the Theme Studio's "Use in this project") and say where it
    /// went. Whether the project may get one (trusted, the theme switched on) is the app's call.
    pub fn write_theme(&self, json: &str) -> Result<PathBuf, ProjectError> {
        let path = self.path(THEME_FILE);
        // Also the gate for the write, as for notes.md: a theme.json that may not be read (a
        // link out of the project) is not replaced either.
        self.read_text(&path, MAX_JSON_BYTES)?;
        write_atomic(&path, json.as_bytes(), LinkPolicy::Within(&self.root))?;
        Ok(path)
    }

    /// The actions, with the keys that can't be used dropped (a [`ProjectSnapshot`] keeps the
    /// warnings that say which).
    pub fn actions(&self) -> Result<Option<Vec<Action>>, ProjectError> {
        let path = self.path(ACTIONS_FILE);
        let Some(mut list) = self.read_json::<Vec<Action>>(&path)? else { return Ok(None) };
        let prompt_ok = |file: &str| self.prompt_file(file).map(drop).map_err(|e| e.reason());
        actions::validate(&mut list, &prompt_ok).map_err(|m| ProjectError::invalid(&path, m))?;
        Ok(Some(list))
    }

    /// Everything this folder says, read once (see [`ProjectSnapshot`]).
    pub fn snapshot(&self) -> ProjectSnapshot {
        ProjectSnapshot::load(self)
    }

    pub fn prompts(&self) -> Result<Option<Vec<Prompt>>, ProjectError> {
        prompts::load_dir(self, &mut |_| {})
    }

    /// The text of `prompts/<file>` (as named by a prompt action's `file`).
    pub fn prompt_file(&self, file: &str) -> Result<String, ProjectError> {
        let path = self.path(PROMPTS_DIR).join(file);
        check_relative(file).map_err(|m| ProjectError::invalid(&path, m))?;
        self.read_text(&path, MAX_PROMPT_BYTES)?.ok_or_else(|| ProjectError::invalid(&path, "does not exist"))
    }

    pub fn layout(&self) -> Result<Option<Layout>, ProjectError> {
        let path = self.path(LAYOUT_FILE);
        let Some(text) = self.read_text(&path, MAX_JSON_BYTES)? else { return Ok(None) };
        parse_layout(&path, &text).map(Some)
    }

    pub fn notes_path(&self) -> PathBuf {
        self.path(NOTES_FILE)
    }

    pub fn notes(&self) -> Result<Option<NotesDoc>, ProjectError> {
        Ok(self.read_text(&self.notes_path(), MAX_NOTES_BYTES)?.map(|text| parse_notes_md(&text)))
    }

    /// Render and write `notes.md` atomically. Returns `false` (and leaves the file alone) when the
    /// content is unchanged, so the app's own file watcher and git don't see spurious edits.
    pub fn write_notes(&self, doc: &NotesDoc) -> Result<bool, ProjectError> {
        let path = self.notes_path();
        let text = render_notes_md(doc);
        // Also the gate for the write: a notes.md that may not be read is not replaced either.
        if self.read_text(&path, MAX_NOTES_BYTES)?.as_deref() == Some(text.as_str()) {
            return Ok(false);
        }
        // The file is in a repository: the write must stay inside it whatever links it ships.
        write_atomic(&path, text.as_bytes(), LinkPolicy::Within(&self.root))?;
        Ok(true)
    }
}

/// `config.json`: settings overrides plus the executable parts, `env` and `shell`.
fn parse_config(path: &Path, text: &str) -> Result<ProjectConfig, ProjectError> {
    config_from_value(path, parse_json(path, text)?)
}

/// [`parse_config`] for the file once it is parsed, which is what a [`ProjectSnapshot`] fingerprints
/// as well: the settings, the environment and the colour all come from the same parse.
fn config_from_value(path: &Path, value: serde_json::Value) -> Result<ProjectConfig, ProjectError> {
    let serde_json::Value::Object(mut settings) = value else {
        return Err(ProjectError::invalid(path, "expected a JSON object"));
    };
    let env = match settings.remove("env") {
        None | Some(serde_json::Value::Null) => BTreeMap::new(),
        Some(env) => serde_json::from_value::<BTreeMap<String, String>>(env)
            .map_err(|e| ProjectError::invalid(path, format!("\"env\" must map names to strings: {e}")))?,
    };
    if let Some(name) = env.keys().find(|k| k.is_empty() || k.contains('=') || k.contains('\0')) {
        return Err(ProjectError::invalid(path, format!("invalid environment variable name {name:?}")));
    }
    let shell = match settings.remove("shell") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) if !s.trim().is_empty() => Some(s),
        Some(_) => return Err(ProjectError::invalid(path, "\"shell\" must be a non-empty string")),
    };
    // A colour that isn't one is set aside, not an error: the rest of the file still counts.
    let (color, color_error) = match settings.remove(color::KEY) {
        None | Some(serde_json::Value::Null) => (None, None),
        Some(serde_json::Value::String(text)) => match ProjectColorChoice::parse(&text) {
            Ok(color) => (Some(color), None),
            Err(why) => (None, Some(why)),
        },
        Some(_) => {
            (None, Some("\"project_color\" must be a string like \"teal\" or \"#2bb3a3\"".to_string()))
        }
    };
    // Like the colour, the lock only decides what the app's menus offer: never a setting.
    let (color_locked, lock_error) = match settings.remove(color::LOCK_KEY) {
        None | Some(serde_json::Value::Null) => (false, None),
        Some(serde_json::Value::Bool(locked)) => (locked, None),
        Some(_) => (false, Some("\"project_color_locked\" must be true or false".to_string())),
    };
    let worktree_setup = match settings.remove("worktree") {
        None | Some(serde_json::Value::Null) => None,
        Some(worktree) => serde_json::from_value::<WorktreeConfig>(worktree)
            .map_err(|e| ProjectError::invalid(path, format!("\"worktree\": {e}")))?
            .setup
            .filter(|setup| !setup.trim().is_empty()),
    };
    // `$schema` is editor tooling, not a setting.
    settings.remove("$schema");
    Ok(ProjectConfig { settings, env, shell, color, color_error, color_locked, lock_error, worktree_setup })
}

/// `config.json`'s `worktree` object. A key this build doesn't know is left alone rather than
/// refused: a project written for a newer Wormhole keeps its `env`, `shell` and colour here.
#[derive(Deserialize)]
struct WorktreeConfig {
    #[serde(default)]
    setup: Option<String>,
}

fn parse_layout(path: &Path, text: &str) -> Result<Layout, ProjectError> {
    let layout = parse_json::<Layout>(path, text)?;
    validate_layout(&layout).map_err(|m| ProjectError::invalid(path, m))?;
    Ok(layout)
}

fn validate_layout(layout: &Layout) -> Result<(), String> {
    let mut names = std::collections::BTreeSet::new();
    for (ix, session) in layout.sessions.iter().enumerate() {
        if session.name.trim().is_empty() {
            return Err(format!("sessions[{ix}]: \"name\" must not be empty"));
        }
        // Names are the handle for `session:<name>` action targets, so they must be unambiguous.
        if !names.insert(session.name.as_str()) {
            return Err(format!("sessions[{ix}]: duplicate name {:?}", session.name));
        }
        if let Some(cwd) = &session.cwd {
            check_relative(cwd).map_err(|m| format!("sessions[{ix}] ({}): \"cwd\" {m}", session.name))?;
        }
    }
    if layout.sessions.iter().filter(|c| c.focus).count() > 1 {
        return Err("only one entry in \"sessions\" may have \"focus\": true".into());
    }
    Ok(())
}

/// Project files are shared through git, so paths in them must stay inside the repository to mean
/// the same thing on every machine.
fn check_relative(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if p.is_absolute() || p.components().any(|c| !matches!(c, Component::Normal(_) | Component::CurDir)) {
        return Err(format!("{path:?} must be a relative path inside the project"));
    }
    Ok(())
}

/// Read a UTF-8 file of at most `cap` bytes; `Ok(None)` if it doesn't exist.
fn read_text(path: &Path, cap: u64, policy: LinkPolicy<'_>) -> Result<Option<String>, ProjectError> {
    match fsutil::read_to_string_capped(path, cap, policy) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(ProjectError::read(path, e)),
    }
}

/// One of the app's own state files in the data dir (a symlink there is the user's own setup).
fn read_state_file(path: &Path) -> Result<Option<String>, ProjectError> {
    read_text(path, MAX_STATE_BYTES, LinkPolicy::Follow)
}

fn parse_json<T: DeserializeOwned>(path: &Path, text: &str) -> Result<T, ProjectError> {
    // If stripping fails (e.g. an unterminated comment), serde reports the error in the raw text.
    let stripped = crate::storage::jsonc::strip(text).unwrap_or_else(|_| text.to_string());
    serde_json::from_str(&stripped).map_err(|e| ProjectError::from_json(path, e))
}

/// Move a state file that doesn't load out of the way (`<name>.bad`, replacing an older one), so
/// saving defaults afterwards doesn't destroy what the user had. Best effort.
fn set_aside(path: &Path) {
    let mut aside = path.as_os_str().to_owned();
    aside.push(".bad");
    match std::fs::rename(path, &aside) {
        Ok(()) => tracing::warn!("{} doesn't load; kept it as {}", path.display(), aside.to_string_lossy()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => tracing::warn!("setting aside {}: {err}", path.display()),
    }
}

/// Replace `path` atomically (see [`crate::storage::fsutil::write_atomic`]: a uniquely named temp file, so
/// nothing planted in a repository can redirect the write).
fn write_atomic(path: &Path, bytes: &[u8], policy: LinkPolicy<'_>) -> Result<(), ProjectError> {
    fsutil::write_atomic(path, bytes, policy).map_err(|e| ProjectError::io(path, e))
}

#[cfg(test)]
mod tests;
