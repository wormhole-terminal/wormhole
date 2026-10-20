//! The context menus in a real window (GPUI's test platform; see `view_tests` for how each test
//! runs in a child process with scratch data). A native menu can't be shown there, so the app
//! records the menu it was asked to open (`native::record`, what devctl's `menu` prints): the
//! tests right-click, read the recorded model and choose an item as a click would.
use gpui_kit::{Modifiers, MouseButton, TestAppContext, VisualTestContext, px, size};
use wormhole_core::sidebar::GroupColor;

use super::app::OpenWindow;
use super::view_tests::{draw, drive, isolate_in, read, start};
use crate::native::record;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn open(cx: &mut TestAppContext, sessions: i64) -> OpenWindow {
    let window = start(cx, sessions);
    cx.simulate_window_resize(window.handle, size(px(1200.0), px(800.0)));
    draw(cx, &window);
    draw(cx, &window);
    window
}

fn right_click(cx: &mut TestAppContext, window: &OpenWindow, selector: &'static str) {
    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    let at = pointer.debug_bounds(selector).expect("the element is drawn").center();
    pointer.simulate_mouse_move(at, None, Modifiers::none());
    pointer.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    pointer.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
}

/// Choose `item` of the open menu and let what it does finish.
fn pick(cx: &mut TestAppContext, window: &OpenWindow, item: &str) {
    let (_, callbacks) = record::pick(item).expect("the item is in the menu");
    cx.update(|cx| {
        window.handle.update(cx, |_, window, cx| {
            for run in callbacks {
                run(window, cx);
            }
        })
    })
    .expect("the window is open");
    cx.run_until_parked();
}

/// A right-click on the + of the title row opens the shells' menu under its heading, and choosing
/// a shell opens a terminal with it.
#[gpui_kit::test]
fn right_clicking_the_plus_offers_the_shells_and_opens_the_chosen_one(cx: &mut TestAppContext) {
    if isolate("right_clicking_the_plus_offers_the_shells_and_opens_the_chosen_one") {
        return;
    }
    let window = open(cx, 1);
    record::close();
    right_click(cx, &window, "new-session-shells");
    let menu = record::open().expect("the right-click opened the menu");
    let outline = menu.outline();
    assert_eq!(outline[0], "# New terminal with");
    assert!(outline.len() > 1 && outline[1..].iter().all(|line| line.starts_with("- ")), "{outline:?}");
    assert_eq!(outline.iter().filter(|line| line.ends_with("[x]")).count(), 1, "the default is checked");

    let before = read(cx, &window, |ws| ws.sessions.len());
    pick(cx, &window, "0");
    assert_eq!(read(cx, &window, |ws| ws.sessions.len()), before + 1, "a terminal opened");
    assert!(record::open().is_none(), "and the menu is closed");
}

/// A right-click on a sidebar row opens its menu and Pin to top pins it; the ring the open menu
/// gives the row goes when the menu closes.
#[gpui_kit::test]
fn right_clicking_a_row_opens_its_menu_and_pin_pins_it(cx: &mut TestAppContext) {
    if isolate("right_clicking_a_row_opens_its_menu_and_pin_pins_it") {
        return;
    }
    let window = open(cx, 2);
    record::close();
    let at = {
        let ix = 0;
        read(cx, &window, move |ws| ws.sidebar.scroll.bounds_for_item(ix).expect("the row is drawn").center())
    };
    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    pointer.simulate_mouse_move(at, None, Modifiers::none());
    pointer.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
    let menu = record::open().expect("the right-click opened the menu");
    assert_eq!(menu.outline().first().map(String::as_str), Some("- Rename…"));
    assert_eq!(read(cx, &window, |ws| ws.sidebar.menu_row), Some(1), "the row wears the ring");

    pick(cx, &window, "Pin to top");
    assert!(read(cx, &window, |ws| ws.sessions[0].pinned), "the terminal is pinned");
    assert_eq!(read(cx, &window, |ws| ws.sidebar.menu_row), None, "the ring went with the menu");
}

/// Let the list's slide finish and draw the window where it lands.
fn settle(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.executor().advance_clock(std::time::Duration::from_secs(2));
    cx.run_until_parked();
    draw(cx, window);
    draw(cx, window);
}

/// A click on a group's name opens its menu (Rename…, the row of colours, the actions); choosing a
/// colour changes the group's, Rename… makes the name a field on the head, and Pin group pins it.
#[gpui_kit::test]
fn clicking_a_groups_name_opens_its_menu_with_colours_and_rename(cx: &mut TestAppContext) {
    if isolate("clicking_a_groups_name_opens_its_menu_with_colours_and_rename") {
        return;
    }
    let window = open(cx, 2);
    let group = cx
        .update(|cx| window.workspace.update(cx, |ws, cx| ws.group_terminals(&[1], cx)))
        .expect("the window is open");
    draw(cx, &window);
    draw(cx, &window);
    record::close();

    record::close();
    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    let at = pointer.debug_bounds("group-label").expect("the group's name is drawn").center();
    pointer.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    let menu = record::open().expect("the click opened the menu");
    assert_eq!(menu.outline().first().map(String::as_str), Some("- Rename…"));
    assert!(menu.outline()[2].starts_with("~ Grey | "), "{:?}", menu.outline());
    assert!(
        read(cx, &window, |ws| ws.sidebar.view.groups.get(group).is_some_and(|g| !g.members.is_empty())),
        "and the click did not fold or close anything"
    );
    assert!(read(cx, &window, |ws| ws.sidebar.group_renaming.is_none()));

    pick(cx, &window, "colour red");
    assert_eq!(
        read(cx, &window, |ws| ws.sidebar.view.groups.get(group).and_then(|g| g.color())),
        Some(GroupColor::Red),
        "the dot set the group's colour"
    );
    assert!(record::open().is_none(), "and closed the menu");

    settle(cx, &window);
    right_click(cx, &window, "group-label");
    assert!(
        record::open().is_some_and(|menu| menu.swatch_row().is_some()),
        "a right-click opens the same menu"
    );
    pick(cx, &window, "Pin group");
    assert!(read(cx, &window, |ws| ws.sidebar.view.groups.get(group).is_some_and(|g| g.pinned)));

    settle(cx, &window);
    right_click(cx, &window, "group-label");
    assert!(record::open().expect("the menu").outline().contains(&"- Unpin group".to_string()));
    pick(cx, &window, "Rename…");
    assert_eq!(read(cx, &window, |ws| ws.sidebar.group_renaming.as_ref().map(|r| r.group)), Some(group));
    draw(cx, &window);
    // The name is a field now: Return keeps what was typed.
    drive(cx, &window, |ws, window, cx| {
        let input = ws.sidebar.group_renaming.as_ref().map(|r| r.input.clone()).expect("renaming");
        input.update(cx, |input, cx| input.set_value("Reviews", window, cx));
        ws.finish_group_rename(true, window, cx);
    });
    assert_eq!(
        read(cx, &window, |ws| ws.sidebar.view.groups.get(group).map(|g| g.name.clone())).as_deref(),
        Some("Reviews")
    );
    assert!(read(cx, &window, |ws| ws.sidebar.group_renaming.is_none()));
}
