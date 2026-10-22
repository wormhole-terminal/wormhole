//! ⇥ and ⇧⇥: F6's order of areas, but stop by stop. Most areas are one stop (a list keeps
//! its own focused row, the tab strip its tab); the title row has one per button, reachable only
//! with macOS Keyboard navigation on, and the panel one per list.

use super::Area;

/// One Tab stop: an area, and which of its stops (the title row's buttons, the panel's lists).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stop {
    pub area: Area,
    pub index: usize,
}

const TERMINAL: Stop = Stop { area: Area::Terminal, index: 0 };

/// The stop ⇥ (`delta` 1) or ⇧⇥ (-1) moves to from `from`, in F6's order, wrapping.
/// `stops(area)` is how many stops the area has right now: 0 when it is hidden or Tab can't reach
/// it (the title row's buttons without Keyboard navigation). The terminal always has one.
/// From nowhere (`None`), as from the terminal. A `from` index past its area's count counts as its
/// last stop. As with F6's `step`, only the sign of `delta` counts, and 0 stays put.
pub fn tab_step(from: Option<Stop>, delta: isize, stops: impl Fn(Area) -> usize) -> Stop {
    // One stop whatever the caller counts: it is always on screen, and is where ⇥ ends up when
    // nothing else is.
    let count = |area: Area| if area == Area::Terminal { 1 } else { stops(area) };
    let from = from.unwrap_or(TERMINAL);
    let here = count(from.area);
    if here > 0 {
        let index = from.index.min(here - 1);
        let inside = match delta.signum() {
            0 => Some(index),
            1 => Some(index + 1).filter(|next| *next < here),
            _ => index.checked_sub(1),
        };
        if let Some(index) = inside {
            return Stop { area: from.area, index };
        }
    } else if delta == 0 {
        return TERMINAL;
    }
    // On to the next area that has a stop, entering it at its near end.
    let all = Area::ALL;
    let len = all.len() as isize;
    let start = all.iter().position(|a| *a == from.area).unwrap_or(3) as isize;
    (1..=len)
        .map(|n| all[(start + delta.signum() * n).rem_euclid(len) as usize])
        .find_map(|area| match count(area) {
            0 => None,
            n => Some(Stop { area, index: if delta < 0 { n - 1 } else { 0 } }),
        })
        .unwrap_or(TERMINAL)
}

#[cfg(test)]
mod tests {
    use super::{Area, Stop, tab_step};

    fn stop(area: Area, index: usize) -> Stop {
        Stop { area, index }
    }

    /// A window as ⇥ sees it: `buttons` title-row buttons (0 without Keyboard navigation), the
    /// sidebar, the panel with `lists` lists (0 when it is closed) and the status bar.
    fn window(buttons: usize, sidebar: bool, lists: usize, status_bar: bool) -> impl Fn(Area) -> usize {
        move |area| match area {
            Area::TitleRow => buttons,
            Area::Sidebar => usize::from(sidebar),
            Area::Tabs | Area::Terminal => 1,
            Area::PanelTabs => usize::from(lists > 0),
            Area::PanelContent => lists,
            Area::StatusBar => usize::from(status_bar),
        }
    }

    /// Every stop ⇥ visits from `from`, once round.
    fn round(from: Stop, delta: isize, stops: impl Fn(Area) -> usize) -> Vec<Stop> {
        let mut at = from;
        let mut seen = Vec::new();
        loop {
            at = tab_step(Some(at), delta, &stops);
            seen.push(at);
            if at == from || seen.len() > 20 {
                return seen;
            }
        }
    }

    #[test]
    fn tab_goes_through_the_regions_in_order_and_back() {
        let everything = window(1, true, 1, true);
        let order = [
            stop(Area::Sidebar, 0),
            stop(Area::Tabs, 0),
            stop(Area::Terminal, 0),
            stop(Area::PanelTabs, 0),
            stop(Area::PanelContent, 0),
            stop(Area::StatusBar, 0),
            stop(Area::TitleRow, 0),
        ];
        assert_eq!(round(stop(Area::TitleRow, 0), 1, &everything), order);
        let mut back: Vec<Stop> = order.iter().rev().skip(1).copied().collect();
        back.push(stop(Area::TitleRow, 0));
        assert_eq!(round(stop(Area::TitleRow, 0), -1, &everything), back);
    }

    #[test]
    fn the_title_rows_buttons_are_stops_only_with_keyboard_navigation() {
        let navigation = window(4, true, 1, true);
        assert_eq!(
            round(stop(Area::StatusBar, 0), 1, &navigation)[..5],
            [
                stop(Area::TitleRow, 0),
                stop(Area::TitleRow, 1),
                stop(Area::TitleRow, 2),
                stop(Area::TitleRow, 3),
                stop(Area::Sidebar, 0)
            ]
        );
        assert_eq!(tab_step(Some(stop(Area::Sidebar, 0)), -1, &navigation), stop(Area::TitleRow, 3));
        assert_eq!(tab_step(Some(stop(Area::TitleRow, 2)), -1, &navigation), stop(Area::TitleRow, 1));

        let without = window(0, true, 1, true);
        assert_eq!(tab_step(Some(stop(Area::StatusBar, 0)), 1, &without), stop(Area::Sidebar, 0));
        assert_eq!(tab_step(Some(stop(Area::Sidebar, 0)), -1, &without), stop(Area::StatusBar, 0));
        assert!(round(stop(Area::Terminal, 0), 1, &without).iter().all(|s| s.area != Area::TitleRow));
    }

    #[test]
    fn each_list_of_the_panel_is_a_stop() {
        // Changes with its Untracked section, under the panel's tabs.
        let changes = window(0, true, 2, true);
        assert_eq!(tab_step(Some(stop(Area::PanelTabs, 0)), 1, &changes), stop(Area::PanelContent, 0));
        assert_eq!(tab_step(Some(stop(Area::PanelContent, 0)), 1, &changes), stop(Area::PanelContent, 1));
        assert_eq!(tab_step(Some(stop(Area::PanelContent, 1)), 1, &changes), stop(Area::StatusBar, 0));
        assert_eq!(tab_step(Some(stop(Area::StatusBar, 0)), -1, &changes), stop(Area::PanelContent, 1));
        assert_eq!(tab_step(Some(stop(Area::PanelContent, 0)), -1, &changes), stop(Area::PanelTabs, 0));
    }

    #[test]
    fn hidden_regions_are_skipped() {
        // The usual window: no status bar; here the sidebar is hidden too.
        let stops = window(0, false, 1, false);
        assert_eq!(
            round(stop(Area::Terminal, 0), 1, &stops),
            [
                stop(Area::PanelTabs, 0),
                stop(Area::PanelContent, 0),
                stop(Area::Tabs, 0),
                stop(Area::Terminal, 0)
            ]
        );
        // The panel closed as well: the strip and the terminal are all there is.
        let bare = window(0, false, 0, false);
        assert_eq!(tab_step(Some(stop(Area::Terminal, 0)), 1, &bare), stop(Area::Tabs, 0));
        assert_eq!(tab_step(Some(stop(Area::Tabs, 0)), -1, &bare), stop(Area::Terminal, 0));
    }

    #[test]
    fn tab_wraps_both_ways() {
        let everything = window(2, true, 3, true);
        assert_eq!(tab_step(Some(stop(Area::StatusBar, 0)), 1, &everything), stop(Area::TitleRow, 0));
        assert_eq!(tab_step(Some(stop(Area::TitleRow, 0)), -1, &everything), stop(Area::StatusBar, 0));
        // Without the status bar the panel's last list wraps to the first button, and back.
        let no_bar = window(2, true, 3, false);
        assert_eq!(tab_step(Some(stop(Area::PanelContent, 2)), 1, &no_bar), stop(Area::TitleRow, 0));
        assert_eq!(tab_step(Some(stop(Area::TitleRow, 0)), -1, &no_bar), stop(Area::PanelContent, 2));
    }

    #[test]
    fn from_nowhere_tab_starts_as_from_the_terminal() {
        let everything = window(1, true, 1, true);
        assert_eq!(tab_step(None, 1, &everything), stop(Area::PanelTabs, 0));
        assert_eq!(tab_step(None, -1, &everything), stop(Area::Tabs, 0));
    }

    #[test]
    fn the_terminal_is_always_one_stop() {
        assert_eq!(tab_step(Some(stop(Area::Sidebar, 0)), 1, |_| 0), stop(Area::Terminal, 0));
        assert_eq!(tab_step(Some(stop(Area::Terminal, 0)), 1, |_| 0), stop(Area::Terminal, 0));
        assert_eq!(tab_step(None, -1, |_| 0), stop(Area::Terminal, 0));
        // Whatever the caller counts for it.
        let three = |area: Area| if area == Area::Terminal { 3 } else { 1 };
        assert_eq!(tab_step(Some(stop(Area::Terminal, 0)), 1, three), stop(Area::PanelTabs, 0));
        assert_eq!(tab_step(Some(stop(Area::Terminal, 2)), -1, three), stop(Area::Tabs, 0));
    }

    #[test]
    fn a_stale_stop_counts_as_its_areas_last() {
        let everything = window(2, true, 3, true);
        // The panel had more lists a moment ago.
        assert_eq!(tab_step(Some(stop(Area::PanelContent, 7)), 1, &everything), stop(Area::StatusBar, 0));
        assert_eq!(tab_step(Some(stop(Area::PanelContent, 7)), -1, &everything), stop(Area::PanelContent, 1));
        // Its area is gone: on from where it was.
        let closed = window(0, true, 0, true);
        assert_eq!(tab_step(Some(stop(Area::PanelContent, 1)), 1, &closed), stop(Area::StatusBar, 0));
        assert_eq!(tab_step(Some(stop(Area::PanelContent, 1)), -1, &closed), stop(Area::Terminal, 0));
        // A button clicked with Keyboard navigation off.
        assert_eq!(tab_step(Some(stop(Area::TitleRow, 1)), 1, &closed), stop(Area::Sidebar, 0));
        assert_eq!(tab_step(Some(stop(Area::TitleRow, 1)), -1, &closed), stop(Area::StatusBar, 0));
    }

    #[test]
    fn only_the_sign_of_delta_counts() {
        let everything = window(2, true, 3, true);
        assert_eq!(tab_step(Some(stop(Area::Sidebar, 0)), 3, &everything), stop(Area::Tabs, 0));
        assert_eq!(tab_step(Some(stop(Area::Sidebar, 0)), -3, &everything), stop(Area::TitleRow, 1));
        assert_eq!(tab_step(Some(stop(Area::PanelContent, 1)), 0, &everything), stop(Area::PanelContent, 1));
        assert_eq!(
            tab_step(Some(stop(Area::PanelContent, 1)), 0, window(0, true, 0, true)),
            stop(Area::Terminal, 0)
        );
    }
}
