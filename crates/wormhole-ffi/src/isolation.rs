//! The experiment never touches the real app's data: it runs only with a data folder of its own
//! (`WORMHOLE_DATA_DIR`, set by the Swift side before anything here is called) whose name says it
//! is the experiment's. Core reads that variable for every path it derives (the data folder, the
//! control socket's name), so the check is on what core will use, not on what Swift believes.

use std::path::Path;

use crate::error::FfiError;

/// What the experiment's data folder is named, as a prefix (`Wormhole-swift`, `Wormhole-swift-test`).
pub(crate) const FOLDER_PREFIX: &str = "Wormhole-swift";

/// Refuse unless core's data folder is the experiment's.
pub(crate) fn require() -> Result<(), FfiError> {
    let dir = wormhole_core::storage::paths::data_dir();
    check(std::env::var_os("WORMHOLE_DATA_DIR").is_some(), &dir)
}

fn check(variable_set: bool, dir: &Path) -> Result<(), FfiError> {
    if !variable_set {
        return Err(FfiError::NotIsolated { reason: "WORMHOLE_DATA_DIR is not set".into() });
    }
    let named =
        dir.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with(FOLDER_PREFIX));
    if named {
        Ok(())
    } else {
        Err(FfiError::NotIsolated {
            reason: format!("{} is not a folder named {FOLDER_PREFIX}*", dir.display()),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::check;

    #[test]
    fn only_the_experiments_own_folder_is_accepted() {
        let real = Path::new("/Users/someone/Library/Application Support/Wormhole");
        assert!(check(true, real).is_err(), "the real app's folder");
        assert!(check(true, Path::new("/tmp/Wormhole-dev")).is_err(), "the dev instance's folder");
        assert!(
            check(false, Path::new("/tmp/Wormhole-swift")).is_err(),
            "no variable: core would use the real one"
        );
        assert!(check(true, Path::new("/tmp/x/Wormhole-swift")).is_ok());
        assert!(check(true, Path::new("/tmp/x/Wormhole-swift-test-7")).is_ok());
    }
}
