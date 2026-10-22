//! The context menus in a real window (GPUI's test platform; see `view_tests` for how each test
//! runs in a child process with scratch data). A native menu can't be shown there, so the app
//! records the menu it was asked to open (`platform::record`, what devctl's `menu` prints): the
//! tests right-click, read the recorded model and choose an item as a click would.
use gpui_kit::{Modifiers, MouseButton, TestAppContext, VisualTestContext, px, size};
use wormhole_core::sidebar::{GroupColor, GroupPaint};

use super::app::OpenWindow;
use super::view_tests::{draw, drive, isolate_in, read, start};
use crate::platform::record;

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

/// Run what a recorded popover command left to run, as its task would after the controls.
fn run(cx: &mut TestAppContext, window: &OpenWindow, callbacks: Vec<crate::platform::menu::Callback>) {
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

fn paint(cx: &mut TestAppContext, window: &OpenWindow, group: u32) -> Option<GroupPaint> {
    read(cx, window, |ws| ws.sidebar.view.groups.get(group).and_then(|g| g.paint()))
}

/// A click on a group's name opens its popover: the name in a field that has the keyboard, the
/// colours, the folder lines and the actions. A dot or a colour of one's own changes the group's colour live (and a
/// custom one is saved with the window), Return keeps a typed name, Esc drops it, and an action
/// does what its menu item does. A right-click on the name opens the native menu instead.
#[gpui_kit::test]
fn clicking_a_groups_name_opens_its_popover_and_a_right_click_its_menu(cx: &mut TestAppContext) {
    if isolate("clicking_a_groups_name_opens_its_popover_and_a_right_click_its_menu") {
        return;
    }
    let window = open(cx, 2);
    let group = cx
        .update(|cx| window.workspace.update(cx, |ws, cx| ws.group_terminals(&[1], cx)))
        .expect("the window is open");
    // The rows slide to their new places; the pointer acts on them once they have.
    settle(cx, &window);
    record::close();

    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    let at = pointer.debug_bounds("group-label").expect("the group's name is drawn").center();
    pointer.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert!(record::open().is_none(), "a click opens no menu");
    let popover = record::open_popover().expect("the click opened the popover");
    let outline = popover.outline();
    assert_eq!(outline[0], "popover transient");
    assert_eq!(outline[1], "field \"Group 1\" (focused)", "the name has the keyboard at once");
    assert!(outline[2].starts_with("colours: No colour | Grey | Blue [x] | "), "{outline:?}");
    assert!(outline[2].ends_with(" | custom"), "the colour wheel after the nine: {outline:?}");
    assert_eq!(
        &outline[3..],
        [
            "---",
            "- Set folder…",
            "---",
            "- New terminal in group",
            "- Ungroup",
            "- Close group",
            "- Pin group",
            "- Move group to new window"
        ]
    );
    assert!(read(cx, &window, |ws| !ws.sidebar.view.groups.get(group).expect("group").members.is_empty()));

    // A dot sets the group's colour while the popover stays.
    run(cx, &window, record::popover_colour("red").expect("a dot"));
    assert_eq!(paint(cx, &window, group), Some(GroupPaint::Named(GroupColor::Red)));
    assert!(record::open_popover().is_some(), "the popover stays open");
    // A colour from the Colours window is the group's own, and it is saved with the window.
    run(cx, &window, record::popover_colour("#336699").expect("the last dot"));
    assert_eq!(paint(cx, &window, group), Some(GroupPaint::Custom([0x33, 0x66, 0x99])));
    let saved = cx
        .update(|cx| window.workspace.read_with(cx, |ws, cx| ws.window_record(cx)))
        .expect("the window is open");
    let layout: super::Layout =
        serde_json::from_str(saved.layout.as_deref().expect("a layout")).expect("it reads back");
    assert_eq!(
        layout.sidebar.groups.get(group).and_then(|g| g.paint()),
        Some(GroupPaint::Custom([0x33, 0x66, 0x99])),
        "the colour of its own is saved with the window"
    );
    run(cx, &window, record::popover_colour("green").expect("a dot"));
    assert_eq!(paint(cx, &window, group), Some(GroupPaint::Named(GroupColor::Green)), "a name replaces it");
    // The empty ring takes the colour away.
    run(cx, &window, record::popover_colour("No colour").expect("the empty ring"));
    assert_eq!(paint(cx, &window, group), None);
    run(cx, &window, record::popover_colour("green").expect("a dot"));

    // Esc drops what was typed; Return keeps it.
    record::popover_type("Nope").expect("the field");
    run(cx, &window, record::popover_key("escape").expect("escape"));
    assert_eq!(
        read(cx, &window, |ws| ws.sidebar.view.groups.get(group).map(|g| g.name.clone())).as_deref(),
        Some("Group 1")
    );
    assert!(record::open_popover().is_none(), "and it closed");
    settle(cx, &window);
    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    pointer.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    record::popover_type("Reviews").expect("the field");
    run(cx, &window, record::popover_key("enter").expect("return"));
    assert_eq!(
        read(cx, &window, |ws| ws.sidebar.view.groups.get(group).map(|g| g.name.clone())).as_deref(),
        Some("Reviews")
    );

    // An action: it closes the popover and does what its menu item does.
    settle(cx, &window);
    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    pointer.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    let (_, callbacks) = record::popover_press("Pin group").expect("an action");
    run(cx, &window, callbacks);
    assert!(read(cx, &window, |ws| ws.sidebar.view.groups.get(group).is_some_and(|g| g.pinned)));
    assert!(record::open_popover().is_none());

    // The right-click is the native menu, with Other colour… and Rename… (in place).
    settle(cx, &window);
    right_click(cx, &window, "group-label");
    let menu = record::open().expect("a right-click opens the menu");
    assert!(menu.swatch_row().is_some());
    assert!(menu.outline().contains(&"- Other colour…".to_string()), "{:?}", menu.outline());
    assert!(menu.outline().contains(&"- Unpin group".to_string()));
    pick(cx, &window, "Rename…");
    assert_eq!(read(cx, &window, |ws| ws.sidebar.group_renaming.as_ref().map(|r| r.group)), Some(group));
    draw(cx, &window);
    // The name is a field now: Return keeps what was typed.
    drive(cx, &window, |ws, window, cx| {
        let input = ws.sidebar.group_renaming.as_ref().map(|r| r.input.clone()).expect("renaming");
        input.update(cx, |input, cx| input.set_value("Ship it", window, cx));
        ws.finish_group_rename(true, window, cx);
    });
    assert_eq!(
        read(cx, &window, |ws| ws.sidebar.view.groups.get(group).map(|g| g.name.clone())).as_deref(),
        Some("Ship it")
    );
    assert!(read(cx, &window, |ws| ws.sidebar.group_renaming.is_none()));
}
