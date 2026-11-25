//! The open-source software the Swift app is built from (`assets/licenses.json`, written by
//! `scripts/gen-licenses.py --ffi` from this workspace's `Cargo.lock`): what the About window's Licenses
//! list shows. It is this library's own list, not core's: the 1.x app links GPUI and not UniFFI. Identical
//! texts are stored once and shared by index, as the file has them.

use anyhow::{Result, ensure};
use serde::Deserialize;

const BUNDLED: &str = include_str!("../../assets/licenses.json");

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LicensePackageRecord {
    pub name: String,
    pub version: String,
    /// The SPDX expression from the crate's `Cargo.toml`.
    pub license: Option<String>,
    pub repository: Option<String>,
    /// Indices into `LicensesRecord::texts`.
    pub texts: Vec<u32>,
    /// A font compiled in, not a crate.
    pub is_font: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LicensesRecord {
    pub texts: Vec<String>,
    pub packages: Vec<LicensePackageRecord>,
}

/// What an entry of the file is. Core's own reader knows only the first two; the icons and logos the Swift app
/// bundles are a third, which is why this library reads the file itself.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    /// A Rust crate from Cargo.lock.
    #[default]
    Crate,
    Font,
    Icons,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) version: String,
    license: Option<String>,
    repository: Option<String>,
    texts: Vec<u32>,
    #[serde(default)]
    pub(crate) kind: Kind,
}

#[derive(Debug, Deserialize)]
pub(crate) struct List {
    texts: Vec<String>,
    pub(crate) crates: Vec<Entry>,
}

impl List {
    pub(crate) fn parse(json: &str) -> Result<List> {
        let list: List = serde_json::from_str(json)?;
        for entry in &list.crates {
            ensure!(
                entry.texts.iter().all(|&index| (index as usize) < list.texts.len()),
                "{} {} points at a license text that does not exist",
                entry.name,
                entry.version
            );
        }
        Ok(list)
    }
}

/// Every package and the license texts shipped in it. Empty (and logged) if the bundled file is broken,
/// which a test rules out.
#[uniffi::export]
pub fn licenses() -> LicensesRecord {
    match List::parse(BUNDLED) {
        Ok(list) => list.into(),
        Err(err) => {
            tracing::error!("the bundled license list is unreadable: {err:#}");
            LicensesRecord { texts: Vec::new(), packages: Vec::new() }
        }
    }
}

impl From<List> for LicensesRecord {
    fn from(list: List) -> Self {
        let packages = list
            .crates
            .into_iter()
            .map(|entry| LicensePackageRecord {
                is_font: entry.kind == Kind::Font,
                name: entry.name,
                version: entry.version,
                license: entry.license,
                repository: entry.repository,
                texts: entry.texts,
            })
            .collect();
        LicensesRecord { texts: list.texts, packages }
    }
}
