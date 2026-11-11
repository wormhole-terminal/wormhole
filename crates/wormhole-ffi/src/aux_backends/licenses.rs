//! The open-source software Wormhole is built from (`update::licenses`, `assets/licenses.json`): what the
//! About window's Licenses list shows. Core's list is the one compiled into the library; identical texts are
//! stored once and shared by index, as the file has them.

use std::collections::HashMap;

use wormhole_core::update::licenses::{Kind, Licenses};

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

/// Every package and the license texts shipped in it.
#[uniffi::export]
pub fn licenses() -> LicensesRecord {
    let list = Licenses::bundled();
    let mut texts: Vec<String> = Vec::new();
    let mut index: HashMap<&str, u32> = HashMap::new();
    let packages = list
        .crates()
        .iter()
        .map(|package| LicensePackageRecord {
            name: package.name.clone(),
            version: package.version.clone(),
            license: package.license.clone(),
            repository: package.repository.clone(),
            texts: list
                .texts_of(package)
                .map(|text| {
                    *index.entry(text).or_insert_with(|| {
                        texts.push(text.to_string());
                        u32::try_from(texts.len() - 1).unwrap_or(u32::MAX)
                    })
                })
                .collect(),
            is_font: package.kind == Kind::Font,
        })
        .collect();
    LicensesRecord { texts, packages }
}
