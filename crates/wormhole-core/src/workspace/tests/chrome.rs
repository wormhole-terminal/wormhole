//! The window's chrome and the search: the sidebar's and panel's visibility, the panel's tab, the
//! overlay, and the filter on the list.

use crate::config::tab::TabId;
use crate::workspace::{Action, Event, Overlay};

use super::{listed, model};

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
