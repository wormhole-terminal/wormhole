//! The context menus in a real window (GPUI's test platform; see `view_tests` for how each test
//! runs in a child process with scratch data). A native menu can't be shown there, so the app
//! records the menu it was asked to open (`native::record`, what devctl's `menu` prints): the
//! tests right-click, read the recorded model and choose an item as a click would.
use gpui_kit::{Modifiers, MouseButton, TestAppContext, VisualTestContext, px, size};

use super::app::OpenWindow;
use super::view_tests::{draw, isolate_in, read, start};
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
