//! Where things are on this machine: the person's home, the folder an app keeps its data in, the per-user
//! temporary folder and the one for runtime files such as sockets. The only place that asks the system or
//! the environment, so the rest of the code says which of the four it means and a port changes it once.

use std::path::PathBuf;

/// The person's home folder: `$HOME` when it is set and not empty (a test instance sets it to a scratch
/// folder and everything it derives from it follows), else the one the system lists for the user.
#[allow(clippy::disallowed_methods, reason = "the one place that asks where home is")]
pub fn home() -> Option<PathBuf> {
    dirs::home_dir()
}

/// The folder an app keeps its data folder in: `~/Library/Application Support` on macOS.
#[allow(clippy::disallowed_methods, reason = "the one place that asks where the data folder is")]
pub fn data_base() -> Option<PathBuf> {
    dirs::data_dir()
}

/// The per-user temporary folder (`$TMPDIR` on macOS, which the system makes for each user), for files
/// that may be left behind.
#[allow(clippy::disallowed_methods, reason = "the one place that asks where the temporary folder is")]
pub fn temp() -> PathBuf {
    std::env::temp_dir()
}

/// Where runtime files such as sockets go: the system's runtime folder where it has one
/// (`$XDG_RUNTIME_DIR`), else the per-user temporary folder, as on macOS.
pub fn runtime() -> PathBuf {
    dirs::runtime_dir().unwrap_or_else(temp)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test instance sets `HOME` to its scratch folder and relies on the app finding everything there.
    #[test]
    fn home_is_what_the_environment_says() {
        match std::env::var_os("HOME").filter(|home| !home.is_empty()) {
            Some(home) => assert_eq!(home_dir_or_panic(), PathBuf::from(home)),
            None => assert!(home().is_none_or(|home| home.is_absolute())),
        }
    }

    fn home_dir_or_panic() -> PathBuf {
        home().expect("HOME is set")
    }

    #[test]
    fn the_data_folder_is_under_home_on_macos() {
        if cfg!(target_os = "macos") {
            assert_eq!(data_base(), home().map(|home| home.join("Library/Application Support")));
        }
    }

    #[test]
    fn runtime_files_have_a_folder_even_without_a_runtime_dir() {
        assert!(runtime().is_absolute());
        assert!(temp().is_absolute());
    }
}
