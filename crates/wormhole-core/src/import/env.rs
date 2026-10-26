//! Where the importer looks: the person's home and the few environment variables the other
//! terminals consult. Everything in this domain finds a file through an [`Env`], so a test points it
//! at a temporary folder and the real home is only ever read through [`Env::from_process`].

use std::path::{Path, PathBuf};

use crate::platform::paths;
use crate::terminal::shells;

/// The places the importer reads from, and the two facts the shell rule needs.
#[derive(Clone, Debug, Default)]
pub struct Env {
    /// The home folder; every other terminal's per-user folder is below it.
    pub home: PathBuf,
    /// `$XDG_CONFIG_HOME`, when it is set to an absolute path.
    pub xdg_config_home: Option<PathBuf>,
    /// `$XDG_CONFIG_DIRS`, which Kitty also searches.
    pub xdg_config_dirs: Vec<PathBuf>,
    /// `$KITTY_CONFIG_DIRECTORY`: Kitty's folder itself, with no `kitty/` below it.
    pub kitty_config_directory: Option<PathBuf>,
    /// `$WEZTERM_CONFIG_FILE`.
    pub wezterm_config_file: Option<PathBuf>,
    /// The folders apps are installed in (`/Applications`, `~/Applications`): Ghostty's bundled
    /// themes are in its app.
    pub applications: Vec<PathBuf>,
    /// `/etc`, where Alacritty looks last.
    pub etc: Option<PathBuf>,
    /// The shell a new terminal runs when nothing else is chosen (`$SHELL`).
    pub login_shell: String,
    /// The shells `/etc/shells` lists that are installed.
    pub system_shells: Vec<String>,
}

impl Env {
    /// An environment that finds only what is under `home`: no variables, no apps, no `/etc`.
    pub fn new(home: PathBuf) -> Self {
        Self { home, ..Self::default() }
    }

    /// This process's own: the real home, the variables, the installed apps. Only the Settings
    /// window and the first-run step call it; devctl and tests build an [`Env`] over a fixture.
    /// `None` when the system names no home folder.
    pub fn from_process() -> Option<Self> {
        let home = paths::home()?;
        let absolute =
            |name: &str| std::env::var_os(name).map(PathBuf::from).filter(|path| path.is_absolute());
        let dirs = std::env::var_os("XDG_CONFIG_DIRS")
            .map(|list| std::env::split_paths(&list).filter(|path| path.is_absolute()).collect())
            .unwrap_or_default();
        Some(Self {
            xdg_config_home: absolute("XDG_CONFIG_HOME"),
            xdg_config_dirs: dirs,
            kitty_config_directory: absolute("KITTY_CONFIG_DIRECTORY"),
            wezterm_config_file: absolute("WEZTERM_CONFIG_FILE"),
            applications: vec![PathBuf::from("/Applications"), home.join("Applications")],
            etc: Some(PathBuf::from("/etc")),
            login_shell: shells::login_shell(),
            system_shells: shells::system_shells().to_vec(),
            home,
        })
    }

    /// `$XDG_CONFIG_HOME`, or `~/.config` when it is not set.
    pub(crate) fn config_home(&self) -> PathBuf {
        self.xdg_config_home.clone().unwrap_or_else(|| self.home.join(".config"))
    }

    /// `~/Library/Application Support`, the Mac's per-user data folder, below this home.
    pub(crate) fn app_support(&self) -> PathBuf {
        self.home.join("Library").join("Application Support")
    }

    /// A path as a config file writes it: `~` and `~/…` are the home folder.
    pub(crate) fn expand_tilde(&self, text: &str) -> PathBuf {
        match text.strip_prefix("~/") {
            Some(rest) => self.home.join(rest),
            None if text == "~" => self.home.clone(),
            None => PathBuf::from(text),
        }
    }

    /// `path` as a file included from `from_dir` names it: `~`, absolute, or relative to the folder
    /// of the file that includes it.
    pub(crate) fn include_path(&self, text: &str, from_dir: &Path) -> PathBuf {
        let path = self.expand_tilde(text);
        if path.is_absolute() { path } else { from_dir.join(path) }
    }
}
