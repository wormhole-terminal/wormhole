//! The window's chrome and the search: the sidebar's and panel's visibility, the panel's tab, the
//! overlay, and the filter on the list.

use crate::config::tab::TabId;
use crate::view::layout::title_parts::TitlePart;
use crate::view::layout::{Edge, Stop};
use crate::view::panes::Preset;
use crate::workspace::{Action, Event, Overlay};

use super::{listed, model, refused};

#[test]
fn toggles_and_tabs_are_chrome_changes_and_asking_for_a_tab_shows_the_panel() {
    let mut m = model(1);
    let before = m.chrome_snapshot();
    assert!(before.sidebar_visible && before.panel_visible);
    assert_eq!(m.apply(Action::ToggleSidebar), [Event::Chrome]);
    assert!(!m.chrome_snapshot().sidebar_visible);
    m.apply(Action::TogglePanel);
    m.apply(Action::ShowPanelTab(TabId::History));
    let after = m.chrome_snapshot();
    assert!(after.panel_visible, "asking for a tab is asking to see it");
    assert_eq!(after.panel_tab, TabId::History);
    assert!(after.revision > before.revision);
    assert!(m.apply(Action::ShowPanelTab(TabId::History)).is_empty(), "no change, no event");
}

#[test]
fn one_overlay_at_a_time() {
    let mut m = model(1);
    m.apply(Action::ShowOverlay(Some(Overlay::Palette)));
    m.apply(Action::ShowOverlay(Some(Overlay::Shortcuts)));
    assert_eq!(m.chrome_snapshot().overlay, Some(Overlay::Shortcuts));
    m.apply(Action::ShowOverlay(None));
    assert_eq!(m.chrome_snapshot().overlay, None);
}

#[test]
fn a_search_lists_the_terminals_it_finds_and_hides_the_sections_it_empties() {
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![3] });
    m.update_facts(1, |f| f.branch = Some("feature/login".into()));
    m.update_facts(2, |f| f.title = Some("vim notes".into()));
    m.apply(Action::Search("login".into()));
    assert_eq!(listed(&m), ["1:1"], "the group with nothing found is not listed; numbers stay put");
    m.apply(Action::Search("VIM  notes".into()));
    assert_eq!(listed(&m), ["2:2"], "every word, any case");
    m.apply(Action::Search(String::new()));
    assert_eq!(listed(&m).len(), 4);
}

#[test]
fn a_new_terminal_ends_a_search_that_would_hide_it() {
    let mut m = model(2);
    m.apply(Action::Search("nothing matches this".into()));
    assert!(listed(&m).is_empty());
    m.apply(Action::NewTerminal);
    assert_eq!(m.chrome_snapshot().search, "");
    assert_eq!(listed(&m).len(), 3);
}

#[test]
fn the_panel_lists_its_tabs_and_a_tab_goes_with_its_feature() {
    let mut m = model(1);
    let tabs = m.chrome_snapshot().tabs;
    assert_eq!(
        tabs.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(),
        ["changes", "history", "notes", "usage", "project", "files"]
    );
    assert!(tabs.iter().all(|t| t.enabled) && tabs.iter().filter(|t| t.selected).count() == 1);
    m.apply(Action::ShowPanelTab(TabId::Notes));
    m.apply(Action::SetFeature { feature: crate::config::FeatureId::Notes, on: false });
    let after = m.chrome_snapshot();
    assert!(!after.tabs[2].enabled, "Notes is off");
    assert_eq!(after.panel_tab, TabId::Changes, "the open tab falls back");
    assert!(refused(&m.apply(Action::ShowPanelTab(TabId::Notes))), "a tab of a feature that is off");
    m.apply(Action::SetFeature { feature: crate::config::FeatureId::Changes, on: false });
    assert!(m.chrome_snapshot().tabs[0].enabled, "an always-on feature has no switch");
}

#[test]
fn the_columns_follow_the_windows_width_and_the_panel_gives_way_first() {
    let mut m = model(1);
    let unknown = m.chrome_snapshot();
    assert_eq!((unknown.window_width, unknown.panel_visible), (0.0, true), "no width, no refusals");
    m.apply(Action::SetWindowWidth(1400.0));
    let wide = m.chrome_snapshot();
    assert_eq!((wide.sidebar_width, wide.panel_width, wide.terminal_width), (350.0, 360.0, 690.0));
    m.apply(Action::SetWindowWidth(700.0));
    let narrow = m.chrome_snapshot();
    assert!(!narrow.panel_visible && narrow.panel_wanted, "wanted, waiting for room");
    assert_eq!(narrow.panel_toggle, crate::view::layout::PanelToggle::Waiting);
    assert_eq!(narrow.panel_width, 0.0);
    // Asking for the panel in a window with no room for it moves the sidebar aside.
    m.apply(Action::ShowPanelTab(TabId::History));
    let shown = m.chrome_snapshot();
    assert!(shown.panel_visible && !shown.sidebar_visible);
    assert_eq!(shown.limits.panel_min, 240.0);
}

#[test]
fn dragging_an_edge_follows_the_pointer_within_the_limits_and_previews_without_moving() {
    let mut m = model(1);
    m.apply(Action::SetWindowWidth(1400.0));
    let preview = m.edge_drag(Edge::Sidebar, 500.0);
    assert_eq!((preview.open, preview.width, preview.stop), (true, 420.0, Some(Stop::Max)));
    assert_eq!(preview.readout, "420 · max");
    assert_eq!(m.chrome_snapshot().sidebar_chosen_width, 350.0, "a preview moves nothing");
    m.apply(Action::DragEdge { edge: Edge::Sidebar, extent: 300.0 });
    assert_eq!(m.chrome_snapshot().sidebar_width, 300.0);
    m.apply(Action::ResetEdge(Edge::Sidebar));
    assert_eq!(m.chrome_snapshot().sidebar_chosen_width, 350.0);
    let hide = m.edge_drag(Edge::Panel, 100.0);
    assert!(!hide.open);
    assert_eq!(hide.readout, "Close");
    m.apply(Action::DragEdge { edge: Edge::Panel, extent: 100.0 });
    assert!(!m.chrome_snapshot().panel_wanted, "dragged past its minimum: closed");
}

#[test]
fn the_layout_menu_says_which_layout_is_on_and_which_have_room() {
    let mut m = model(1);
    let free = m.chrome_snapshot().layout;
    assert_eq!(
        free.presets.iter().map(|p| p.title.as_str()).collect::<Vec<_>>(),
        ["2 Columns", "2 Rows", "2×2 Grid", "Single Pane"]
    );
    assert!(free.presets.iter().all(|p| p.enabled) && free.can_split_row && free.can_split_column);
    assert!(free.presets[3].current, "a terminal on its own is a single pane");
    m.apply(Action::SetPaneArea { width: 300.0, height: 500.0 });
    let tight = m.chrome_snapshot().layout;
    assert!(!tight.can_split_row, "150 points is under the least a pane may be");
    assert!(tight.can_split_column);
    assert!(!tight.presets[0].enabled && !tight.presets[2].enabled && tight.presets[1].enabled);
}

#[test]
fn a_layout_makes_the_terminals_it_needs_and_a_smaller_one_lets_the_rest_go() {
    let mut m = model(1);
    m.apply(Action::SetPaneArea { width: 1600.0, height: 900.0 });
    let events = m.apply(Action::ApplyLayout(Preset::Grid));
    assert_eq!(events.iter().filter(|e| matches!(e, Event::Opened { .. })).count(), 3);
    let chrome = m.chrome_snapshot();
    assert!(chrome.layout.presets.iter().find(|p| p.preset == Preset::Grid).is_some_and(|p| p.current));
    assert_eq!((chrome.layout.pane_count, chrome.selected), (4, Some(1)));
    let panes = m.panes_snapshot(1600.0, 900.0);
    assert_eq!(panes.panes.len(), 4);
    assert_eq!(panes.dividers.len(), 3, "one down the middle, one in each column");
    m.apply(Action::ApplyLayout(Preset::Single));
    assert_eq!(m.panes_snapshot(1600.0, 900.0).panes.len(), 1);
    assert_eq!(m.ids().len(), 4, "the others keep running, as rows of their own");
    assert_eq!(listed(&m).len(), 4);
}

#[test]
fn a_divider_moves_and_equalizes() {
    let mut m = model(1);
    m.apply(Action::SetPaneArea { width: 1000.0, height: 600.0 });
    m.apply(Action::SplitBeside { anchor: 1, axis: crate::view::panes::Axis::Row });
    let divider = m.panes_snapshot(1000.0, 600.0).dividers.remove(0);
    assert_eq!(divider.position, 500.0);
    let events =
        m.apply(Action::DragDivider { path: divider.path.clone(), index: divider.index, position: 700.0 });
    assert!(events.contains(&Event::Layout));
    assert_eq!(m.panes_snapshot(1000.0, 600.0).dividers[0].position, 700.0);
    m.apply(Action::EqualizeDivider { path: divider.path, index: divider.index });
    assert_eq!(m.panes_snapshot(1000.0, 600.0).dividers[0].position, 500.0);
    assert!(refused(&m.apply(Action::DragDivider { path: vec![9], index: 0, position: 1.0 })));
}

#[test]
fn the_title_row_lists_its_parts_chevrons_and_path() {
    let mut m = model(1);
    let bare = m.title_snapshot("Macintosh HD");
    assert!(bare.left.iter().all(|slot| !slot.chevron_before), "nothing to join yet");
    m.update_facts(1, |f| {
        f.cwd = Some("/Users/me/Projects/app".into());
        f.branch = Some("main".into());
    });
    let title = m.title_snapshot("Macintosh HD");
    let present: Vec<_> =
        title.left.iter().filter(|s| s.present).map(|s| (s.part, s.chevron_before)).collect();
    assert_eq!(
        present,
        [
            (TitlePart::Folder, false),
            (TitlePart::Branch, true),
            (TitlePart::BranchStatus, false),
            (TitlePart::Terminal, true)
        ]
    );
    assert_eq!(
        title.right.iter().filter(|s| s.present).map(|s| s.part).collect::<Vec<_>>(),
        [TitlePart::OpenIn, TitlePart::Splits]
    );
    assert_eq!(
        title.path_menu.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["app", "Projects", "me", "Users", "Macintosh HD"]
    );
    assert_eq!(title.terminal.as_ref().and_then(|t| t.branch.as_deref()), Some("main"));
    // Without a home folder the path is as it is; whole first, then losing its middle.
    assert_eq!(
        title.path_steps.iter().map(|s| format!("{}{}", s.parents, s.last)).collect::<Vec<_>>(),
        ["/Users/me/Projects/app", "/Users/…/Projects/app", "/Users/…/app"]
    );
    // The person hides the branch and moves the splits to the left end.
    m.apply(Action::SetTitleRow(vec![TitlePart::Splits, TitlePart::Folder, TitlePart::Terminal]));
    let arranged = m.title_snapshot("Macintosh HD");
    assert_eq!(
        arranged.left.iter().map(|s| s.part).collect::<Vec<_>>(),
        [TitlePart::Splits, TitlePart::Folder, TitlePart::Terminal]
    );
    assert!(arranged.hidden.contains(&TitlePart::Branch));
}

#[test]
fn the_title_rows_last_chip_names_what_runs_not_what_the_person_called_it() {
    let mut m = model(1);
    m.update_facts(1, |f| f.shell = "zsh".into());
    m.apply(Action::Rename { id: 1, name: "api".into() });
    let name =
        |m: &crate::workspace::WorkspaceModel| m.title_snapshot("Macintosh HD").terminal.map(|t| t.name);
    assert_eq!(name(&m).as_deref(), Some("zsh"), "the program's name, as the GPUI row's chip is");
    assert_eq!(m.sidebar_snapshot(0).sections[0].rows[0].name, "api", "the list keeps the person's");
}
