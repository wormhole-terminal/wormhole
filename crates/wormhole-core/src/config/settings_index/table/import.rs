//! The Import section: the terminals found on this Mac with what each would bring over, and how they
//! compare with Wormhole. Each group is one block the window draws itself.

use super::{entry, group};
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const IMPORT: &[Group] = &[
    group(
        SETTINGS_IMPORT_GROUP_FOUND,
        &[entry("import-found", SETTINGS_IMPORT_FOUND, &[]).with_keywords(SETTINGS_IMPORT_FOUND_KEYWORDS)],
    ),
    group(
        SETTINGS_IMPORT_GROUP_COMPARE,
        &[entry("import-compare", SETTINGS_IMPORT_COMPARE, &[])
            .with_keywords(SETTINGS_IMPORT_COMPARE_KEYWORDS)],
    ),
];
