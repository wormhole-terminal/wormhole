//! The Swift app never opens the 1.x app's data. Core reads `WORMHOLE_DATA_DIR` for every path it derives (the data
//! folder, the control socket's name) and falls back to 1.x's own folder, `Wormhole`, without it, so every entry of
//! this library checks what core will use, not what the caller believes, and the variable must be set.
//!
//! Two kinds of folder are accepted, one per identity of the app (`macos/swift/Sources/Wormhole/Isolation.swift` is
//! the Swift half of the same rule):
//!
//! - a folder named `Wormhole-swift*`: dev builds and tests, whose keys are in files and whose homes are confined;
//! - the beta's own, [`BETA_FOLDER`], and only in a library that keeps no keys in files. A library that does (a dev
//!   build) must never be given the folder of a person's real beta: its sealed stores would keep their keys beside
//!   what they protect, and a shipped library that wrongly kept the `dev-key` feature fails at its first launch
//!   instead of doing so quietly.

use std::path::Path;

use crate::error::FfiError;

/// What a dev or test data folder is named, as a prefix (`Wormhole-swift`, `Wormhole-swift-test`).
const FOLDER_PREFIX: &str = "Wormhole-swift";

/// The beta's data folder in Application Support, whole name (`DATA_DIR_NAME` in `packaging/beta.env`, which a test
/// compares with it).
pub(crate) const BETA_FOLDER: &str = "Wormhole Beta";

/// Whether this library keeps its keys in files instead of the Keychain: what core decides with the same two
/// conditions (`storage::secret_key::uses_dev_key`: a debug build, or its `dev-key` feature, which this crate's own
/// turns on). Core's third condition, its own test build, never holds in a library linked from here.
const KEYS_IN_FILES: bool = cfg!(any(debug_assertions, feature = "dev-key"));

/// Refuse unless core's data folder is one this library may open.
pub(crate) fn require() -> Result<(), FfiError> {
    let dir = wormhole_core::storage::paths::data_dir();
    check(std::env::var_os("WORMHOLE_DATA_DIR").is_some(), &dir, KEYS_IN_FILES)
}

/// Refuse unless `dir`, a folder an object was given instead of core's own (tests and previews pass theirs), is one
/// this library may open: the same rule as the variable's.
pub(crate) fn require_folder(dir: &Path) -> Result<(), FfiError> {
    folder(dir, KEYS_IN_FILES)
}

/// The rule as a pure function: `variable_set` says whether `WORMHOLE_DATA_DIR` names the folder at all (without it
/// core would use 1.x's), `dir` is the folder core will use, `keys_in_files` is what the library is.
fn check(variable_set: bool, dir: &Path, keys_in_files: bool) -> Result<(), FfiError> {
    if !variable_set {
        return Err(FfiError::NotIsolated { reason: "WORMHOLE_DATA_DIR is not set".into() });
    }
    folder(dir, keys_in_files)
}

fn folder(dir: &Path, keys_in_files: bool) -> Result<(), FfiError> {
    let name = dir.file_name().and_then(|name| name.to_str());
    let reason = match name {
        Some(name) if name.starts_with(FOLDER_PREFIX) => return Ok(()),
        Some(BETA_FOLDER) if keys_in_files => {
            format!(
                "{} is the beta's folder, which a library that keeps its keys in files never opens",
                dir.display()
            )
        }
        Some(BETA_FOLDER) => return Ok(()),
        _ => format!("{} is not a folder named {FOLDER_PREFIX}* or {BETA_FOLDER}", dir.display()),
    };
    Err(FfiError::NotIsolated { reason })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{BETA_FOLDER, check, folder, require_folder};

    const SUPPORT: &str = "/Users/someone/Library/Application Support";

    fn accepted(variable_set: bool, name: &str, keys_in_files: bool) -> bool {
        check(variable_set, &Path::new(SUPPORT).join(name), keys_in_files).is_ok()
    }

    /// Every row of the rule, for a library that keeps keys in files (dev builds, tests) and one that does not
    /// (the shipped build), as (variable set, folder name, dev library accepts, shipped library accepts).
    #[test]
    fn the_whole_table() {
        let rows: &[(bool, &str, bool, bool)] = &[
            // Without the variable core would use 1.x's folder: refused whatever the name it would have been.
            (false, "Wormhole-swift", false, false),
            (false, "Wormhole Beta", false, false),
            (false, "Wormhole", false, false),
            // A dev or test folder: either library.
            (true, "Wormhole-swift", true, true),
            (true, "Wormhole-swift-test-7", true, true),
            (true, "Wormhole-swift-owner", true, true),
            // The beta's folder: only the shipped library, whole name only.
            (true, "Wormhole Beta", false, true),
            (true, "Wormhole Beta 2", false, false),
            (true, "Wormhole Beta-swift", false, false),
            (true, "wormhole beta", false, false),
            (true, "WormholeBeta", false, false),
            // Anything else, 1.x's folder and the other tools' first.
            (true, "Wormhole", false, false),
            (true, "Wormhole-dev", false, false),
            (true, "Wormhole-Swift", false, false),
            (true, "wormhole-swift", false, false),
            (true, "Andromeda", false, false),
        ];
        for &(set, name, dev, shipped) in rows {
            assert_eq!(accepted(set, name, true), dev, "dev library, variable set {set}, {name:?}");
            assert_eq!(accepted(set, name, false), shipped, "shipped library, variable set {set}, {name:?}");
        }
    }

    /// The library as built, which a run per configuration covers (`cargo test`: refused; `cargo test --release
    /// --no-default-features`, the shipped one: accepted).
    #[test]
    fn this_build_opens_the_betas_folder_only_without_dev_key() {
        let beta = Path::new(SUPPORT).join(BETA_FOLDER);
        assert_eq!(require_folder(&beta).is_ok(), !cfg!(any(debug_assertions, feature = "dev-key")));
    }

    #[test]
    fn the_folder_is_the_last_component_and_only_a_folder_with_a_name_counts() {
        let beta = Path::new(SUPPORT).join(BETA_FOLDER);
        // A trailing separator names the same folder.
        assert!(folder(Path::new(&format!("{}/", beta.display())), false).is_ok());
        // The beta's name is not enough as a parent of something else, nor as a file's directory.
        assert!(folder(&beta.join("Wormhole"), false).is_err());
        assert!(folder(&beta.join("Wormhole-swift/.."), false).is_err(), "no file name");
        assert!(folder(Path::new("/"), true).is_err());
        assert!(folder(Path::new(""), true).is_err());
    }

    #[test]
    fn a_refusal_says_which_folder_and_why() {
        let beta = Path::new(SUPPORT).join(BETA_FOLDER);
        let dev = folder(&beta, true).expect_err("refused").to_string();
        assert!(dev.contains(BETA_FOLDER) && dev.contains("keeps its keys in files"), "{dev}");
        let real = folder(&Path::new(SUPPORT).join("Wormhole"), false).expect_err("refused").to_string();
        assert!(real.contains("Application Support/Wormhole ") && real.contains("Wormhole-swift*"), "{real}");
        let unset = check(false, &beta, false).expect_err("refused").to_string();
        assert!(unset.contains("WORMHOLE_DATA_DIR is not set"), "{unset}");
    }

    /// The beta's folder name is `DATA_DIR_NAME` of `packaging/beta.env`, and no other identity's name is.
    #[test]
    fn the_beta_folder_is_the_one_in_the_packaging_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging");
        let value = |file: &str, key: &str| -> String {
            let text = std::fs::read_to_string(path.join(file)).expect("the packaging file");
            let line = text.lines().find_map(|line| line.strip_prefix(&format!("{key}="))).expect("the key");
            line.trim().trim_matches('"').to_owned()
        };
        assert_eq!(value("beta.env", "DATA_DIR_NAME"), BETA_FOLDER);
        assert_ne!(value("app.env", "DATA_DIR_NAME"), BETA_FOLDER, "1.x's folder is never the beta's");
    }
}
