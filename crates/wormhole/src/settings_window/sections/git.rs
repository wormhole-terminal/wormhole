//! Changes & diffs: the Changes panel's file view and untracked group, the diff's
//! default layout, wrapping, tab width and the units file sizes are written in.

use gpui_kit::assets::IconName;
use wormhole_core::config::SizeUnits;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::whole;

use crate::settings_window::registry::{Segment, Setting, toggle};

pub(super) const SETTINGS: &[Setting] = &[
    Setting::segmented(
        "changes-view",
        &[Segment::text(SETTINGS_GIT_LIST), Segment::text(SETTINGS_GIT_TREE)],
        |s| usize::from(s.changes_tree),
        |s, ix| s.changes_tree = ix == 1,
    ),
    toggle!("group-untracked", group_untracked),
    Setting::segmented(
        "diff-layout",
        &[
            Segment::icon(IconName::Columns2, SETTINGS_GIT_SPLIT),
            Segment::icon(IconName::Rows2, SETTINGS_GIT_UNIFIED),
        ],
        |s| usize::from(!s.diff_split),
        |s, ix| s.diff_split = ix == 0,
    ),
    toggle!("diff-wrap", diff_wrap),
    Setting::stepper(
        "tab-width",
        |s| settings_git_tab_width_value(whole(s.tab_width)),
        |s, dir| s.tab_width = s.tab_width.saturating_add_signed(dir as isize),
    ),
    Setting::segmented(
        "size-units",
        &[Segment::text(SETTINGS_GIT_MB), Segment::text(SETTINGS_GIT_MIB)],
        |s| usize::from(s.size_units == SizeUnits::Binary),
        |s, ix| s.size_units = if ix == 1 { SizeUnits::Binary } else { SizeUnits::Decimal },
    ),
];
