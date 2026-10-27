//! The title row's parts as a person names them (core's `layout::title_parts` arranges them): each
//! one's name and icon, shared with Settings' list of them, and the menu a right-click on the row
//! opens, which shows or hides each of them in place.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::view::layout::title_parts::{TitlePart, TitleRow};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{Key, text};

use crate::actions::RevealSetting;
use crate::platform::{self, Item, Menu};
use crate::settings::{edits_in, store_in, update_settings};
use crate::window::Workspace;

/// The settings row that arranges the parts (core's `settings_index`).
pub(crate) const SETTING: &str = "title-row";

/// A part's name.
pub(crate) fn label(part: TitlePart) -> Key {
    match part {
        TitlePart::Folder => TITLE_ROW_PART_FOLDER,
        TitlePart::Branch => TITLE_ROW_PART_BRANCH,
        TitlePart::BranchStatus => TITLE_ROW_PART_BRANCH_STATUS,
        TitlePart::Terminal => TITLE_ROW_PART_TERMINAL,
        TitlePart::TerminalStatus => TITLE_ROW_PART_TERMINAL_STATUS,
        TitlePart::Space => TITLE_ROW_PART_SPACE,
        TitlePart::AgentGauge => TITLE_ROW_PART_AGENT_GAUGE,
        TitlePart::Ports => TITLE_ROW_PART_PORTS,
        TitlePart::PullRequest => TITLE_ROW_PART_PULL_REQUEST,
        TitlePart::OpenIn => TITLE_ROW_PART_OPEN_IN,
        TitlePart::Splits => TITLE_ROW_PART_SPLITS,
    }
}

/// The icon Settings' list shows before a part's name: the glyph the row draws it with, where it
/// has one.
pub(crate) fn icon(part: TitlePart) -> IconName {
    match part {
        TitlePart::Folder => IconName::Folder,
        TitlePart::Branch => IconName::GitBranch,
        TitlePart::BranchStatus => IconName::GitCommitHorizontal,
        TitlePart::Terminal => IconName::Terminal,
        TitlePart::TerminalStatus => IconName::Clock,
        TitlePart::Space => IconName::ArrowRightLeft,
        TitlePart::AgentGauge => IconName::Gauge,
        TitlePart::Ports => IconName::Globe,
        TitlePart::PullRequest => IconName::GitPullRequest,
        TitlePart::OpenIn => IconName::ExternalLink,
        TitlePart::Splits => IconName::Columns2,
    }
}

impl Workspace {
    /// The row's own menu, under the pointer at `at`: every part with a check while it is shown.
    pub(in crate::window) fn open_title_menu(&self, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
        // Under Zen mode the row on screen is Zen's, not the person's: the menu would check the wrong
        // parts and change the wrong ones. It shows their own, and leaves them be until Zen mode ends.
        let store = store_in(&self.profile, cx);
        let row = store.settings_without_zen().title_row().into_owned();
        let editable = edits_in(&self.profile, cx) && !store.zen();
        platform::show_menu(row_menu(&row, editable), at, window, cx);
    }
}

/// The menu of the title row arranged as `row`: each part (in the default order, so a part keeps its
/// place in the menu wherever it sits in the row) with a check while it is shown, a reset, and
/// Settings' list, where they are also moved. A window of a profile that isn't the one shown can't
/// change its parts here, nor open them in Settings (which shows the shown profile's): they are off.
fn row_menu(row: &TitleRow, editable: bool) -> Menu {
    let parts = TitlePart::ALL.into_iter().filter(|part| part.hideable());
    let menu = parts.fold(Menu::new().header(text(TITLE_ROW_MENU_HEADER)), |menu, part| {
        let shown = row.shows(part);
        menu.item(
            Item::new(text(label(part)))
                .checked(shown)
                .enabled(editable)
                .on_select(move |_, cx| update_settings(cx, |s| s.show_title_part(part, !shown))),
        )
    });
    menu.separator()
        .item(
            Item::new(text(TITLE_ROW_MENU_RESET))
                .enabled(editable && !row.is_default())
                .on_select(|_, cx| update_settings(cx, |s| s.title_row = Some(TitleRow::default()))),
        )
        .item(
            Item::new(text(TITLE_ROW_MENU_CUSTOMIZE))
                .enabled(editable)
                .on_select(|window, cx| window.dispatch_action(Box::new(RevealSetting(SETTING)), cx)),
        )
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::layout::title_parts::{TitlePart, TitleRow};

    use super::row_menu;

    #[test]
    fn the_menu_checks_the_parts_shown_in_their_default_order() {
        let mut row = TitleRow::default();
        row.set_shown(TitlePart::Ports, false);
        row.move_part(0, 4);
        assert_eq!(
            row_menu(&row, true).outline(),
            [
                "# Show in title row",
                "- Folder [x]",
                "- Branch [x]",
                "- Branch status [x]",
                "- Terminal name [x]",
                "- Terminal status [x]",
                "- Agent model and context [x]",
                "- Open ports",
                "- Pull request [x]",
                "- Open in… [x]",
                "- Split buttons [x]",
                "---",
                "- Reset title row",
                "- Customize title row…",
            ]
        );
    }

    #[test]
    fn a_window_of_another_profile_changes_nothing_from_here() {
        let outline = row_menu(&TitleRow::default(), false).outline();
        assert!(outline[1..=10].iter().all(|item| item.ends_with("[x] [off]")), "{outline:?}");
        assert_eq!(outline[12..], ["- Reset title row [off]", "- Customize title row… [off]"]);
    }
}
