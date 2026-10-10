//! The areas of a window that keyboard focus moves between, in their order: F6 goes to
//! the next one that is on screen, ⇧F6 to the one before, wrapping around, even from the
//! terminal (which keeps ⇥ for its program). Outside the terminal ⇥ walks the same order stop by
//! stop (`tab`), Esc climbs back to the terminal (`escape`), and inside a list the arrow keys and
//! type-ahead move its one focused item (`list`).

mod escape;
pub mod list;
mod tab;

pub use escape::{EscState, Escape, escape};
pub use tab::{Stop, tab_step};

/// One of a window's areas, in focus order (its regions 1–7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Area {
    /// The title row's buttons: hide the sidebar, Settings, the filter, a new terminal.
    TitleRow,
    Sidebar,
    /// The terminal's title strip (its breadcrumb): ← → step to the previous or next terminal.
    Tabs,
    Terminal,
    PanelTabs,
    PanelContent,
    /// The optional status bar along the bottom.
    StatusBar,
}

impl Area {
    pub const ALL: [Area; 7] = [
        Area::TitleRow,
        Area::Sidebar,
        Area::Tabs,
        Area::Terminal,
        Area::PanelTabs,
        Area::PanelContent,
        Area::StatusBar,
    ];

    /// Its name in `devctl state`.
    pub fn name(self) -> &'static str {
        match self {
            Area::TitleRow => "title-row",
            Area::Sidebar => "sidebar",
            Area::Tabs => "tabs",
            Area::Terminal => "terminal",
            Area::PanelTabs => "panel-tabs",
            Area::PanelContent => "panel-content",
            Area::StatusBar => "status-bar",
        }
    }
}

/// The area F6 (`delta` 1) or ⇧F6 (`delta` -1) moves to from `from`, skipping the ones not on
/// screen. Focus nowhere in particular counts as the terminal's. The terminal is always on
/// screen, so there is always somewhere to go.
pub fn step(from: Option<Area>, delta: isize, shown: impl Fn(Area) -> bool) -> Area {
    let all = Area::ALL;
    let len = all.len() as isize;
    let start = all.iter().position(|a| Some(*a) == from.or(Some(Area::Terminal))).unwrap_or(3) as isize;
    (1..=len)
        .map(|n| all[(start + delta.signum() * n).rem_euclid(len) as usize])
        .find(|area| *area == Area::Terminal || shown(*area))
        .unwrap_or(Area::Terminal)
}

#[cfg(test)]
mod tests {
    use super::{Area, step};

    #[test]
    fn f6_goes_round_the_areas_on_screen() {
        let everything = |_: Area| true;
        let mut at = Area::Terminal;
        let mut seen = Vec::new();
        for _ in 0..7 {
            at = step(Some(at), 1, everything);
            seen.push(at);
        }
        assert_eq!(
            seen,
            [
                Area::PanelTabs,
                Area::PanelContent,
                Area::StatusBar,
                Area::TitleRow,
                Area::Sidebar,
                Area::Tabs,
                Area::Terminal
            ]
        );
        assert_eq!(step(Some(Area::TitleRow), -1, everything), Area::StatusBar, "⇧F6 wraps backwards");
    }

    #[test]
    fn hidden_areas_are_skipped() {
        // The usual window: the status bar is off; here the panel is closed too.
        let shown = |a: Area| !matches!(a, Area::StatusBar | Area::PanelTabs | Area::PanelContent);
        assert_eq!(step(Some(Area::Terminal), 1, shown), Area::TitleRow);
        assert_eq!(step(Some(Area::TitleRow), -1, shown), Area::Terminal);
        assert_eq!(step(None, 1, shown), Area::TitleRow, "from nowhere, as from the terminal");
        assert_eq!(step(Some(Area::Sidebar), 1, |_| false), Area::Terminal, "the terminal is always there");
    }
}
