//! Bringing settings over from other terminals (core `import`): which of them have settings on this Mac, what
//! each would change, and applying what is ticked. Every file is read capped and nothing of the source is
//! written (core's rules); `home` names the folder to look under, `None` is the person's own.

use std::path::PathBuf;

use wormhole_core::config::Config;
use wormhole_core::import::{self, Env, Proposed, Source};
use wormhole_core::storage::paths::DataRoot;
use wormhole_text::text;

use super::common::{AuxError, resolve_data_dir};

/// One setting a source says and Wormhole would take.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ImportRowRecord {
    /// The setting's stable key (`font_size`, `theme`).
    pub setting: String,
    /// What the source wrote, as it wrote it.
    pub from: String,
    /// Ticked from the start.
    pub default_on: bool,
}

/// A terminal with settings on this Mac.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ImportFoundRecord {
    /// The stable name (`ghostty`, `iterm2`).
    pub source: String,
    /// Its name in the interface.
    pub name: String,
    pub files: Vec<String>,
    pub rows: Vec<ImportRowRecord>,
    /// How many keys of it Wormhole cannot take.
    pub skipped: u32,
}

fn env_of(home: Option<String>) -> Result<Env, AuxError> {
    match home {
        Some(home) => Ok(Env::new(PathBuf::from(home))),
        None => Env::from_process().ok_or_else(|| AuxError::failed("the system names no home folder")),
    }
}

/// Every terminal with settings under `home`, the one changed last first. Blocks (reads files): off the main
/// thread.
#[uniffi::export]
pub fn import_scan(home: Option<String>) -> Result<Vec<ImportFoundRecord>, AuxError> {
    let found = import::scan(&env_of(home)?);
    Ok(found
        .into_iter()
        .map(|found| ImportFoundRecord {
            source: found.source.key().to_string(),
            name: text(found.source.name_key()).to_string(),
            files: found.files.iter().map(|f| f.display().to_string()).collect(),
            rows: found
                .proposal
                .values
                .iter()
                .map(|p| ImportRowRecord {
                    setting: p.setting.key().to_string(),
                    from: p.from.clone(),
                    default_on: p.default_on,
                })
                .collect(),
            skipped: u32::try_from(found.proposal.skipped.len()).unwrap_or(u32::MAX),
        })
        .collect())
}

/// Applies `source`'s settings to the Default profile's `config.json`: the rows named in `ticked` (their
/// setting keys), or the ones ticked by default when `None`. Palettes are saved as theme files first. How
/// many settings were taken.
#[uniffi::export]
pub fn import_apply(
    source: String,
    home: Option<String>,
    ticked: Option<Vec<String>>,
    data_dir: Option<String>,
) -> Result<u32, AuxError> {
    let profile = DataRoot::new(resolve_data_dir(data_dir)?).default_profile();
    let source = Source::from_key(&source).ok_or_else(|| AuxError::failed("no such terminal"))?;
    let found = import::scan_one(&env_of(home)?, source)
        .ok_or_else(|| AuxError::failed("it has no settings on this Mac"))?;
    let chosen: Vec<Proposed> = found
        .proposal
        .values
        .iter()
        .filter(|p| match &ticked {
            Some(keys) => keys.iter().any(|key| key == p.setting.key()),
            None => p.default_on,
        })
        .cloned()
        .collect();
    let saved = import::save_themes(&profile.themes_dir(), &chosen).map_err(AuxError::failed)?;
    Config::load_for(&profile).update(|settings| import::apply(settings, &found.proposal, &chosen, &saved));
    Ok(u32::try_from(chosen.len()).unwrap_or(u32::MAX))
}
