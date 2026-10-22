//! One profile: who it is and what its terminals are given.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::agents::AgentId;
use crate::storage::paths::ProfileId;
use crate::view::sidebar::GroupColor;

/// What a terminal's environment says about its profile (`WORMHOLE_PROFILE=<id>`): for scripts and
/// prompts. Hooks do not use it; they report a terminal id, and the app knows whose terminal that is.
pub const PROFILE_VAR: &str = "WORMHOLE_PROFILE";

const MAX_NAME: usize = 40;
const MAX_ENV: usize = 64;
const MAX_VALUE: usize = 4096;
const MAX_FOLDERS: usize = 64;
const MAX_IDENTITY: usize = 200;

/// Why a profile or the list was refused, or couldn't be read or saved.
#[derive(Debug)]
pub enum ProfileError {
    /// A value that can't be used; the message is for the person who typed it.
    Invalid(String),
    /// No profile has this id.
    Unknown(String),
    /// Deleting the Default profile.
    IsDefault,
    /// `profiles.json` doesn't parse.
    Parse {
        path: PathBuf,
        message: String,
    },
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl ProfileError {
    pub(super) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    pub(super) fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io { path: path.to_path_buf(), source }
    }
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Unknown(id) => write!(f, "there is no profile \"{id}\""),
            Self::IsDefault => f.write_str("the Default profile can't be deleted"),
            Self::Parse { path, message } => write!(f, "{}: {message}", path.display()),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for ProfileError {}

/// The name and email git records in a terminal's commits. Set only in that terminal's
/// environment; Wormhole never writes them to a repository or to git's config.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GitIdentity {
    pub name: String,
    pub email: String,
}

/// Where one coding agent keeps its login for this profile (`~/.claude-work`), handed to the
/// agent through its own variable ([`login_env_var`]). Never copied to another profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentLogin {
    pub agent: AgentId,
    /// Absolute, or starting with `~/`.
    pub dir: String,
}

/// The variable that points `agent` at a different login folder; `None` for an agent that has no
/// such setting (it keeps one login per user).
pub fn login_env_var(agent: AgentId) -> Option<&'static str> {
    match agent {
        AgentId::Claude => Some("CLAUDE_CONFIG_DIR"),
        AgentId::Codex => Some("CODEX_HOME"),
        _ => None,
    }
}

/// A profile as `profiles.json` has it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    #[serde(with = "color_key", default = "default_color")]
    pub color: GroupColor,
    /// The one letter in the chip and the avatar; the name's first letter when the file has none.
    #[serde(with = "letter", default)]
    pub avatar: char,
    /// Where new terminals start (absolute or `~/…`); the home folder when empty.
    #[serde(default)]
    pub start_dir: String,
    /// Folders (absolute or `~/…`) that open in this profile too.
    #[serde(default)]
    pub folders: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub git: GitIdentity,
    #[serde(default)]
    pub agent_logins: Vec<AgentLogin>,
}

fn default_color() -> GroupColor {
    GroupColor::Blue
}

impl Profile {
    pub fn new(id: ProfileId, name: &str, color: GroupColor) -> Self {
        let name = name.trim().to_string();
        let avatar = first_letter(&name);
        Self {
            id,
            name,
            color,
            avatar,
            start_dir: String::new(),
            folders: Vec::new(),
            env: BTreeMap::new(),
            git: GitIdentity::default(),
            agent_logins: Vec::new(),
        }
    }

    /// The Default profile as a first launch has it.
    pub fn default_profile() -> Self {
        Self::new(ProfileId::default_profile(), "Default", GroupColor::Blue)
    }

    /// Everything a person typed, checked: the first thing wrong is the error.
    pub fn check(&self) -> Result<(), ProfileError> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME || name.chars().any(char::is_control) {
            return Err(ProfileError::invalid(format!("A profile's name is 1 to {MAX_NAME} characters")));
        }
        if !self.avatar.is_alphanumeric() {
            return Err(ProfileError::invalid("A profile's avatar is one letter or digit"));
        }
        if !self.start_dir.is_empty() && !is_folder(&self.start_dir) {
            return Err(ProfileError::invalid("The start folder is a full path, or starts with ~/"));
        }
        if self.folders.len() > MAX_FOLDERS || !self.folders.iter().all(|f| is_folder(f)) {
            return Err(ProfileError::invalid("Each folder is a full path, or starts with ~/"));
        }
        if self.env.len() > MAX_ENV {
            return Err(ProfileError::invalid(format!("At most {MAX_ENV} variables")));
        }
        if let Some((key, _)) = self.env.iter().find(|(k, v)| !valid_env(k, v)) {
            return Err(ProfileError::invalid(format!("{key} can't be set in a profile")));
        }
        if !valid_identity(&self.git.name) || !valid_identity(&self.git.email) {
            return Err(ProfileError::invalid("The git name and email are one line each"));
        }
        for login in &self.agent_logins {
            if login_env_var(login.agent).is_none() || !is_folder(&login.dir) {
                return Err(ProfileError::invalid(format!(
                    "{:?} can't have a login folder here",
                    login.agent
                )));
            }
        }
        let mut seen: Vec<AgentId> = Vec::new();
        for login in &self.agent_logins {
            if seen.contains(&login.agent) {
                return Err(ProfileError::invalid("Each agent has one login folder"));
            }
            seen.push(login.agent);
        }
        Ok(())
    }

    /// Bring a profile read from the file into shape: what [`check`](Self::check) would refuse is
    /// dropped (or replaced) instead of failing the whole file, since one bad entry must not take
    /// every profile with it. Returns whether anything was changed.
    pub(super) fn sanitize(&mut self) -> bool {
        let before = self.clone();
        let name: String = self.name.trim().chars().filter(|c| !c.is_control()).take(MAX_NAME).collect();
        self.name = if name.is_empty() { self.id.to_string() } else { name };
        if !self.avatar.is_alphanumeric() {
            self.avatar = first_letter(&self.name);
        }
        if !self.start_dir.is_empty() && !is_folder(&self.start_dir) {
            self.start_dir.clear();
        }
        self.folders.retain(|f| is_folder(f));
        self.folders.truncate(MAX_FOLDERS);
        self.env.retain(|k, v| valid_env(k, v));
        while self.env.len() > MAX_ENV {
            self.env.pop_last();
        }
        if !valid_identity(&self.git.name) {
            self.git.name.clear();
        }
        if !valid_identity(&self.git.email) {
            self.git.email.clear();
        }
        let mut seen: Vec<AgentId> = Vec::new();
        self.agent_logins.retain(|l| {
            let keep = login_env_var(l.agent).is_some() && is_folder(&l.dir) && !seen.contains(&l.agent);
            seen.push(l.agent);
            keep
        });
        *self != before
    }

    /// The variables this profile's terminals get, for a terminal's [`SpawnOptions`](crate::terminal::SpawnOptions):
    /// the git identity, the agents' login folders, then the environment the person wrote, which
    /// has the last word over the first two.
    pub fn terminal_env(&self) -> BTreeMap<String, String> {
        self.terminal_env_with_home(dirs::home_dir().as_deref())
    }

    pub(super) fn terminal_env_with_home(&self, home: Option<&Path>) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        for (var, value) in [
            ("GIT_AUTHOR_NAME", &self.git.name),
            ("GIT_COMMITTER_NAME", &self.git.name),
            ("GIT_AUTHOR_EMAIL", &self.git.email),
            ("GIT_COMMITTER_EMAIL", &self.git.email),
        ] {
            if !value.trim().is_empty() {
                env.insert(var.to_string(), value.trim().to_string());
            }
        }
        for login in &self.agent_logins {
            let (Some(var), Some(dir)) = (login_env_var(login.agent), expand_home(&login.dir, home)) else {
                continue;
            };
            env.insert(var.to_string(), dir.to_string_lossy().into_owned());
        }
        env.extend(self.env.iter().map(|(k, v)| (k.clone(), v.clone())));
        env
    }

    /// The login folders of this profile's agents, `~` resolved against `home` (the ones that
    /// can't be are left out): where Wormhole's status hooks go for those logins.
    pub fn login_dirs(&self, home: Option<&Path>) -> Vec<(AgentId, PathBuf)> {
        self.agent_logins
            .iter()
            .filter(|login| login_env_var(login.agent).is_some())
            .filter_map(|login| Some((login.agent, expand_home(&login.dir, home)?)))
            .collect()
    }

    /// Where new terminals start, if the profile says.
    pub fn start_folder(&self) -> Option<PathBuf> {
        if self.start_dir.is_empty() {
            None
        } else {
            expand_home(&self.start_dir, dirs::home_dir().as_deref())
        }
    }

    /// Whether `path` is inside one of the folders that open in this profile, and how deep that
    /// folder is (the deeper one wins when two profiles claim a path).
    pub(super) fn claim_depth(&self, path: &Path, home: Option<&Path>) -> Option<usize> {
        self.folders
            .iter()
            .filter_map(|f| expand_home(f, home))
            .filter(|folder| path.starts_with(folder))
            .map(|folder| folder.components().count())
            .max()
    }
}

/// `~/x` against `home`, or an absolute path as it is; `None` for anything else.
pub(super) fn expand_home(text: &str, home: Option<&Path>) -> Option<PathBuf> {
    if text == "~" {
        return home.map(Path::to_path_buf);
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return home.map(|home| home.join(rest));
    }
    text.starts_with('/').then(|| PathBuf::from(text))
}

fn is_folder(text: &str) -> bool {
    let ok_shape = text == "~" || text.starts_with("~/") || text.starts_with('/');
    ok_shape && text.len() <= MAX_VALUE && !text.contains('\0') && !text.split('/').any(|part| part == "..")
}

fn valid_identity(text: &str) -> bool {
    text.chars().count() <= MAX_IDENTITY && !text.chars().any(char::is_control)
}

/// A variable a profile may set: a plain name, a value without NUL, and none of Wormhole's own
/// (the terminal's id, socket and the like are not for a profile to redirect).
fn valid_env(key: &str, value: &str) -> bool {
    let mut chars = key.chars();
    let name_ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    let own = key.starts_with("WORMHOLE_") || key.starts_with("ANDROMEDA_");
    name_ok && !own && key.len() <= 128 && value.len() <= MAX_VALUE && !value.contains('\0')
}

/// The name's first letter or digit, upper-cased; `P` for a name that has none.
/// The avatar letter a name gets when none is chosen: its first letter or digit, upper case.
pub fn first_letter(name: &str) -> char {
    name.chars().find(|c| c.is_alphanumeric()).and_then(|c| c.to_uppercase().next()).unwrap_or('P')
}

/// The colour as its name in the file; a name this build doesn't know is blue, not an error.
mod color_key {
    use serde::{Deserialize, Deserializer, Serializer};

    use super::default_color;
    use crate::view::sidebar::GroupColor;

    pub fn serialize<S: Serializer>(color: &GroupColor, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(color.key())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<GroupColor, D::Error> {
        let key = String::deserialize(deserializer)?;
        Ok(GroupColor::from_key(&key).unwrap_or_else(default_color))
    }
}

/// The avatar as a one-character string; anything else reads as "none yet" (a space, then
/// [`Profile::sanitize`] picks the name's letter).
mod letter {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(letter: &char, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&letter.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<char, D::Error> {
        let text = String::deserialize(deserializer)?;
        let mut chars = text.chars();
        Ok(match (chars.next(), chars.next()) {
            (Some(c), None) => c,
            _ => ' ',
        })
    }
}
