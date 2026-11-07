//! Motion: a select for each thing that moves, as any other row. The page's preview strip loops through
//! the choice of the row touched last (`motion_preview`): hovering a row or opening its menu shows its
//! scene, and so does changing it. Which animation each choice names is core's (`view::motion`); the
//! words of the options are in the catalogs. Picking an option writes `motion.<choice>` in `config.json`
//! and the preview starts over with it.

use gpui_kit::*;
use wormhole_core::view::motion::{ColumnMotion, FoldMotion, InsertMotion, MotionChoice, StatusMotion};
use wormhole_text::Key;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::MotionScene;
use crate::select::Choice;
use crate::settings::Settings;
use crate::settings_window::SettingsWindow;
use crate::settings_window::registry::Setting;
use crate::style;

/// What the options of a choice are called, in the order the menu lists them.
type Labels<T> = &'static [(T, Key)];

const INSERT: Labels<InsertMotion> = &[
    (InsertMotion::Classic, SETTINGS_MOTION_CLASSIC),
    (InsertMotion::Reveal, SETTINGS_MOTION_INSERT_REVEAL),
    (InsertMotion::FromSide, SETTINGS_MOTION_INSERT_FROM_SIDE),
    (InsertMotion::SoftSpring, SETTINGS_MOTION_INSERT_SOFT_SPRING),
];
const FOLD: Labels<FoldMotion> = &[
    (FoldMotion::Classic, SETTINGS_MOTION_CLASSIC),
    (FoldMotion::Drawer, SETTINGS_MOTION_FOLD_DRAWER),
    (FoldMotion::Tuck, SETTINGS_MOTION_FOLD_TUCK),
];
const STATUS: Labels<StatusMotion> = &[
    (StatusMotion::Classic, SETTINGS_MOTION_CLASSIC),
    (StatusMotion::Crossfade, SETTINGS_MOTION_STATUS_CROSSFADE),
    (StatusMotion::BadgePop, SETTINGS_MOTION_STATUS_BADGE_POP),
    (StatusMotion::Wash, SETTINGS_MOTION_STATUS_WASH),
    (StatusMotion::Flare, SETTINGS_MOTION_STATUS_FLARE),
    (StatusMotion::Nudge, SETTINGS_MOTION_STATUS_NUDGE),
    (StatusMotion::LineRoll, SETTINGS_MOTION_STATUS_LINE_ROLL),
];
const COLUMN: Labels<ColumnMotion> = &[
    (ColumnMotion::Classic, SETTINGS_MOTION_CLASSIC),
    (ColumnMotion::SoftSpring, SETTINGS_MOTION_COLUMN_SOFT_SPRING),
    (ColumnMotion::ContentAfter, SETTINGS_MOTION_COLUMN_CONTENT_AFTER),
];

/// What a select stores for `field`: the option the menu picked, by the name `config.json` spells.
macro_rules! pick {
    ($field:ident) => {
        |s: &mut Settings, name: &str| {
            if let Some(choice) = MotionChoice::from_name(name) {
                s.motion.$field = choice;
            }
        }
    };
}

pub(super) const SETTINGS: &[Setting] = &[
    Setting::select(
        "motion-insert",
        style::SELECT_W,
        |_| choices(INSERT),
        |s| s.motion.insert.name().to_string(),
        pick!(insert),
    )
    .touched(|w, cx| w.show_motion(MotionScene::Insert, cx)),
    Setting::select(
        "motion-fold",
        style::SELECT_W,
        |_| choices(FOLD),
        |s| s.motion.fold.name().to_string(),
        pick!(fold),
    )
    .touched(|w, cx| w.show_motion(MotionScene::Fold, cx)),
    Setting::select(
        "motion-status",
        style::SELECT_W,
        |_| choices(STATUS),
        |s| s.motion.status.name().to_string(),
        pick!(status),
    )
    .touched(|w, cx| w.show_motion(MotionScene::Status, cx)),
    Setting::select(
        "motion-column",
        style::SELECT_W,
        |_| choices(COLUMN),
        |s| s.motion.column.name().to_string(),
        pick!(column),
    )
    .touched(|w, cx| w.show_motion(MotionScene::Column, cx)),
];

impl SettingsWindow {
    /// The preview shows `scene`: a row of its was touched. Nothing, when the page's preview is not up
    /// (a search's results hold these rows too).
    pub(in crate::settings_window) fn show_motion(&mut self, scene: MotionScene, cx: &mut Context<Self>) {
        if let Some(strip) = &self.motion_strip
            && strip.scene(cx) != scene
        {
            strip.show(scene, cx);
            cx.notify();
        }
    }
}

fn choices<T: MotionChoice>(labels: Labels<T>) -> Vec<Choice<String>> {
    labels.iter().map(|(choice, key)| Choice::new(choice.name().to_string(), text(*key))).collect()
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::motion::{ColumnMotion, FoldMotion, InsertMotion, MotionChoice, StatusMotion};
    use wormhole_text::text;

    use super::{COLUMN, FOLD, INSERT, Labels, STATUS};
    use crate::settings::Settings;

    /// The menu lists every option of the choice once, in core's order, each with words.
    fn lists_every_option<T: MotionChoice + std::fmt::Debug>(labels: Labels<T>) {
        let listed: Vec<T> = labels.iter().map(|(choice, _)| *choice).collect();
        assert_eq!(listed, T::ALL, "the menu and the options differ");
        for (choice, key) in labels {
            assert!(!text(*key).is_empty(), "{choice:?} has no name");
        }
    }

    #[test]
    fn every_menu_lists_all_the_options_of_its_choice() {
        lists_every_option(INSERT);
        lists_every_option(FOLD);
        lists_every_option(STATUS);
        lists_every_option(COLUMN);
    }

    #[test]
    fn picking_by_name_sets_that_choice_alone() {
        let mut s = Settings::default();
        let pick = pick!(status);
        pick(&mut s, "badge-pop");
        assert_eq!(s.motion.status, StatusMotion::BadgePop);
        pick(&mut s, "no-such-option");
        assert_eq!(s.motion.status, StatusMotion::BadgePop, "a name that is no option changes nothing");
        assert_eq!(
            (s.motion.insert, s.motion.fold, s.motion.column),
            (InsertMotion::FromSide, FoldMotion::Tuck, ColumnMotion::Classic)
        );
    }
}
