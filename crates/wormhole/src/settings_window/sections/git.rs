//! Changes & diffs: the Changes panel's file view and untracked group, the diff's
//! default layout, wrapping, tab width and the units file sizes are written in.

use wormhole_core::config::SizeUnits;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::whole;

use crate::settings_window::registry::{Current, MenuOption, Setting, toggle};

/// The tab widths the menu offers, the usual one first.
const TAB_WIDTHS: [u32; 3] = [4, 2, 8];

/// "4 spaces".
fn spaces(width: u32) -> String {
    settings_git_tab_width_value(whole(width))
}

pub(super) const SETTINGS: &[Setting] = &[
    Setting::menu(
        "changes-view",
        &[MenuOption::text(SETTINGS_GIT_LIST), MenuOption::text(SETTINGS_GIT_TREE)],
        |s| usize::from(s.changes_tree),
        |s, ix| s.changes_tree = ix == 1,
    ),
    toggle!("group-untracked", group_untracked),
    Setting::menu(
        "diff-layout",
        &[MenuOption::text(SETTINGS_GIT_SPLIT), MenuOption::text(SETTINGS_GIT_UNIFIED)],
        |s| usize::from(!s.diff_split),
        |s, ix| s.diff_split = ix == 0,
    ),
    toggle!("diff-wrap", diff_wrap),
    Setting::menu_of_values(
        "tab-width",
        &[
            MenuOption::counted(spaces, TAB_WIDTHS[0]),
            MenuOption::counted(spaces, TAB_WIDTHS[1]),
            MenuOption::counted(spaces, TAB_WIDTHS[2]),
        ],
        // A width written in config.json that the menu doesn't list shows as it is.
        |s| match TAB_WIDTHS.iter().position(|&width| width as usize == s.tab_width) {
            Some(at) => Current::At(at),
            None => Current::Other(settings_git_tab_width_value(whole(s.tab_width))),
        },
        |s, ix| s.tab_width = TAB_WIDTHS[ix.min(TAB_WIDTHS.len() - 1)] as usize,
    ),
    Setting::menu(
        "size-units",
        &[MenuOption::text(SETTINGS_GIT_MB), MenuOption::text(SETTINGS_GIT_MIB)],
        |s| usize::from(s.size_units == SizeUnits::Binary),
        |s, ix| s.size_units = if ix == 1 { SizeUnits::Binary } else { SizeUnits::Decimal },
    ),
];

#[cfg(test)]
mod tests {
    use wormhole_core::config::Settings;

    use super::SETTINGS;
    use crate::settings_window::registry::{Control, Current, Menu};

    fn menu() -> &'static Menu {
        SETTINGS
            .iter()
            .find_map(|s| match &s.control {
                Control::Menu(menu) if s.id == "tab-width" => Some(menu),
                _ => None,
            })
            .expect("the tab width menu")
    }

    fn shown(width: usize) -> String {
        match menu().current(&Settings { tab_width: width, ..Settings::default() }) {
            Current::At(at) => format!("option {at}"),
            Current::Other(words) => words,
        }
    }

    #[test]
    fn the_menu_offers_four_two_and_eight() {
        assert_eq!([shown(4), shown(2), shown(8)], ["option 0", "option 1", "option 2"]);
        let mut s = Settings::default();
        for (ix, width) in [4, 2, 8].into_iter().enumerate() {
            (menu().set)(&mut s, ix);
            assert_eq!(s.tab_width, width);
        }
    }

    #[test]
    fn a_width_written_by_hand_shows_as_it_is_and_stays_until_another_is_chosen() {
        assert_eq!(shown(3), "3 spaces");
        assert_eq!(shown(1), "1 space");
        let s = Settings { tab_width: 3, ..Settings::default() };
        let _ = menu().current(&s);
        assert_eq!(s.tab_width, 3, "reading never writes");
    }
}
