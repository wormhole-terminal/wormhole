//! The icon theme file format (Zed's v0.3.0 schema). Tolerant: unknown fields
//! such as `"$schema"` are ignored, so Zed's files load unchanged.

use std::collections::HashMap;

use serde::Deserialize;

use super::{Appearance, RenderMode};

#[derive(Deserialize)]
pub(super) struct FamilyContent {
    pub(super) name: String,
    #[serde(default)]
    pub(super) author: String,
    /// `x-andromeda` is what the extension was called up to 0.2: still read in themes made then.
    #[serde(default, rename = "x-wormhole", alias = "x-andromeda")]
    pub(super) wormhole: WormholeExt,
    #[serde(default)]
    pub(super) themes: Vec<ThemeContent>,
}

#[derive(Deserialize, Default)]
pub(super) struct WormholeExt {
    #[serde(default)]
    pub(super) render: RenderMode,
}

#[derive(Deserialize)]
pub(super) struct ThemeContent {
    pub(super) name: String,
    pub(super) appearance: Appearance,
    #[serde(default)]
    pub(super) directory_icons: PairContent,
    #[serde(default)]
    pub(super) named_directory_icons: HashMap<String, PairContent>,
    #[serde(default)]
    pub(super) chevron_icons: PairContent,
    #[serde(default)]
    pub(super) file_stems: HashMap<String, String>,
    #[serde(default)]
    pub(super) file_suffixes: HashMap<String, String>,
    #[serde(default)]
    pub(super) file_icons: HashMap<String, IconDefContent>,
}

#[derive(Deserialize, Default)]
pub(super) struct PairContent {
    pub(super) collapsed: Option<String>,
    pub(super) expanded: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct IconDefContent {
    pub(super) path: String,
}
