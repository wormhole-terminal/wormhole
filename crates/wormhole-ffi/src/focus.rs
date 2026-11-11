//! Keyboard focus (`wormhole_core::view::focus`) and how much a window shows (`config::presets`), as pure
//! functions: the shell holds the real focus and the window's state, hands the facts over and acts on the
//! answer. F6's order, ⇥'s stops, the Esc ladder, how a list is walked and Zen mode's layer are core's;
//! nothing here keeps state.

use wormhole_core::config::presets::{Preset, Shown};
use wormhole_core::view::focus::{self, Area, EscState, Escape, Stop, list};

/// One of a window's areas, in focus order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FocusArea {
    TitleRow,
    Sidebar,
    Tabs,
    Terminal,
    PanelTabs,
    PanelContent,
    StatusBar,
}

impl From<Area> for FocusArea {
    fn from(area: Area) -> Self {
        match area {
            Area::TitleRow => Self::TitleRow,
            Area::Sidebar => Self::Sidebar,
            Area::Tabs => Self::Tabs,
            Area::Terminal => Self::Terminal,
            Area::PanelTabs => Self::PanelTabs,
            Area::PanelContent => Self::PanelContent,
            Area::StatusBar => Self::StatusBar,
        }
    }
}

impl From<FocusArea> for Area {
    fn from(area: FocusArea) -> Self {
        match area {
            FocusArea::TitleRow => Self::TitleRow,
            FocusArea::Sidebar => Self::Sidebar,
            FocusArea::Tabs => Self::Tabs,
            FocusArea::Terminal => Self::Terminal,
            FocusArea::PanelTabs => Self::PanelTabs,
            FocusArea::PanelContent => Self::PanelContent,
            FocusArea::StatusBar => Self::StatusBar,
        }
    }
}

/// One ⇥ stop: an area and which of its stops (the title row's buttons, the panel's lists).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FocusStop {
    pub area: FocusArea,
    pub index: u32,
}

/// How many ⇥ stops an area has right now (0 when hidden or unreachable by Tab).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct AreaStops {
    pub area: FocusArea,
    pub count: u32,
}

/// The area F6 (`delta` 1) or ⇧F6 (-1) goes to; `shown` lists the areas on screen.
#[uniffi::export]
pub fn focus_step(from: Option<FocusArea>, delta: i32, shown: Vec<FocusArea>) -> FocusArea {
    focus::step(from.map(Into::into), delta as isize, |area| shown.contains(&area.into())).into()
}

/// The stop ⇥ (`delta` 1) or ⇧⇥ (-1) goes to, in F6's order, stop by stop.
#[uniffi::export]
pub fn focus_tab_step(from: Option<FocusStop>, delta: i32, stops: Vec<AreaStops>) -> FocusStop {
    let from = from.map(|s| Stop { area: s.area.into(), index: s.index as usize });
    let to = focus::tab_step(from, delta as isize, |area| {
        stops.iter().find(|s| Area::from(s.area) == area).map_or(0, |s| s.count as usize)
    });
    FocusStop { area: to.area.into(), index: to.index as u32 }
}

/// Where things stand when Esc is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct EscapeState {
    /// The terminal has the keyboard.
    pub terminal: bool,
    /// A menu, popover or rename field is open.
    pub popup: bool,
    /// A diff or a find bar is open over the terminal.
    pub detail: bool,
}

/// The rung of the Esc ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum EscapeRung {
    ToShell,
    ClosePopup,
    CloseDetail,
    ToTerminal,
}

#[uniffi::export]
pub fn focus_escape(state: EscapeState) -> EscapeRung {
    match focus::escape(EscState { terminal: state.terminal, popup: state.popup, detail: state.detail }) {
        Escape::ToShell => EscapeRung::ToShell,
        Escape::ClosePopup => EscapeRung::ClosePopup,
        Escape::CloseDetail => EscapeRung::CloseDetail,
        Escape::ToTerminal => EscapeRung::ToTerminal,
    }
}

/// One item of a list as the keyboard walks it (only items on screen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct ListItem {
    pub level: u8,
    /// `Some(open)` for one that folds.
    pub fold: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ListKey {
    Up,
    Down,
    Left,
    Right,
    /// ⌥↑
    First,
    /// ⌥↓
    Last,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ListMove {
    To { index: u32 },
    Open { index: u32 },
    Fold { index: u32 },
    Stay,
}

/// What `key` does with the focus on `at` (`view::focus::list::walk`).
#[uniffi::export]
pub fn focus_list_walk(items: Vec<ListItem>, at: Option<u32>, key: ListKey) -> ListMove {
    let items: Vec<list::Item> = items.iter().map(|i| list::Item { level: i.level, fold: i.fold }).collect();
    let key = match key {
        ListKey::Up => list::Key::Up,
        ListKey::Down => list::Key::Down,
        ListKey::Left => list::Key::Left,
        ListKey::Right => list::Key::Right,
        ListKey::First => list::Key::First,
        ListKey::Last => list::Key::Last,
    };
    match list::walk(&items, at.map(|a| a as usize), key) {
        list::Move::To(index) => ListMove::To { index: index as u32 },
        list::Move::Open(index) => ListMove::Open { index: index as u32 },
        list::Move::Fold(index) => ListMove::Fold { index: index as u32 },
        list::Move::Stay => ListMove::Stay,
    }
}

/// Whether Space (not ↩) is pressed: only Space folds or opens a band.
#[uniffi::export]
pub fn focus_press_folds_band(space: bool) -> bool {
    list::band_folds(if space { list::Press::Space } else { list::Press::Return })
}

/// The item type-ahead jumps to for `typed` (`view::focus::list::type_ahead`).
#[uniffi::export]
pub fn focus_type_ahead(names: Vec<String>, at: Option<u32>, typed: String) -> Option<u32> {
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    list::type_ahead(&names, at.map(|a| a as usize), &typed).map(|i| i as u32)
}

/// A way to fill in how much the window shows at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WindowPreset {
    Zen,
    Balanced,
    Everything,
}

/// How much a window shows under a preset (`config::presets::Shown`, without the tools).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PresetShown {
    /// The title row's parts as `config.json` names them (`title_row`).
    pub title_row: Vec<String>,
    pub status_bar: bool,
    pub minimap: bool,
    pub column_dividers: bool,
    /// Whether the right panel starts open: a window's own state, not a setting.
    pub panel_open: bool,
}

#[uniffi::export]
pub fn window_preset(preset: WindowPreset) -> PresetShown {
    let shown = Shown::of(match preset {
        WindowPreset::Zen => Preset::Zen,
        WindowPreset::Balanced => Preset::Balanced,
        WindowPreset::Everything => Preset::Everything,
    });
    let title_row = shown
        .title
        .row()
        .parts()
        .iter()
        .filter_map(|part| serde_json::to_value(part).ok()?.as_str().map(str::to_owned))
        .collect();
    PresetShown {
        title_row,
        status_bar: shown.status_bar,
        minimap: shown.minimap,
        column_dividers: shown.column_dividers,
        panel_open: shown.panel_open,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERYTHING: [FocusArea; 7] = [
        FocusArea::TitleRow,
        FocusArea::Sidebar,
        FocusArea::Tabs,
        FocusArea::Terminal,
        FocusArea::PanelTabs,
        FocusArea::PanelContent,
        FocusArea::StatusBar,
    ];

    #[test]
    fn f6_walks_the_shown_areas_and_wraps() {
        let shown = vec![FocusArea::Sidebar, FocusArea::Tabs];
        let next = focus_step(Some(FocusArea::Terminal), 1, shown.clone());
        assert_eq!(
            next,
            FocusArea::Sidebar,
            "the title row and the panel are not shown, so F6 wraps to the sidebar"
        );
        assert_eq!(focus_step(Some(FocusArea::Sidebar), -1, shown), FocusArea::Terminal);
        assert_eq!(focus_step(None, 1, EVERYTHING.to_vec()), FocusArea::PanelTabs);
    }

    #[test]
    fn tab_enters_the_title_row_button_by_button() {
        let stops: Vec<AreaStops> = EVERYTHING
            .iter()
            .map(|&area| AreaStops { area, count: if area == FocusArea::TitleRow { 2 } else { 1 } })
            .collect();
        let from = Some(FocusStop { area: FocusArea::StatusBar, index: 0 });
        let first = focus_tab_step(from, 1, stops.clone());
        assert_eq!(first, FocusStop { area: FocusArea::TitleRow, index: 0 });
        let second = focus_tab_step(Some(first), 1, stops);
        assert_eq!(second, FocusStop { area: FocusArea::TitleRow, index: 1 });
    }

    #[test]
    fn the_esc_ladder_goes_nearest_first() {
        let state = |terminal, popup, detail| focus_escape(EscapeState { terminal, popup, detail });
        assert_eq!(state(false, true, true), EscapeRung::ClosePopup);
        assert_eq!(state(false, false, true), EscapeRung::CloseDetail);
        assert_eq!(
            state(true, false, true),
            EscapeRung::CloseDetail,
            "a find bar closes before the shell hears Esc"
        );
        assert_eq!(state(false, false, false), EscapeRung::ToTerminal);
        assert_eq!(state(true, false, false), EscapeRung::ToShell);
    }

    #[test]
    fn a_list_walks_and_types_ahead() {
        let items = vec![
            ListItem { level: 0, fold: Some(true) },
            ListItem { level: 1, fold: None },
            ListItem { level: 0, fold: Some(false) },
        ];
        assert_eq!(focus_list_walk(items.clone(), Some(0), ListKey::Left), ListMove::Fold { index: 0 });
        assert_eq!(focus_list_walk(items.clone(), Some(1), ListKey::Left), ListMove::To { index: 0 });
        assert_eq!(focus_list_walk(items, None, ListKey::Up), ListMove::To { index: 2 });
        assert!(focus_press_folds_band(true) && !focus_press_folds_band(false));
        let names = vec!["api".to_string(), "server".to_string(), "scratch".to_string()];
        assert_eq!(focus_type_ahead(names.clone(), Some(0), "s".into()), Some(1));
        assert_eq!(
            focus_type_ahead(names.clone(), Some(1), "ss".into()),
            Some(2),
            "one letter again steps on"
        );
        assert_eq!(
            focus_type_ahead(names, Some(2), "scr".into()),
            Some(2),
            "spelled out, the name keeps the focus"
        );
    }

    #[test]
    fn zen_shows_the_name_and_the_splits_and_no_panel() {
        let zen = window_preset(WindowPreset::Zen);
        assert_eq!(zen.title_row, ["terminal", "space", "splits"]);
        assert!(!zen.status_bar && !zen.minimap && !zen.column_dividers && !zen.panel_open);
        assert!(window_preset(WindowPreset::Everything).status_bar);
    }
}
