//! The ticks (400 ms; the app's and each window's) leave the globals the views read alone while nothing changed: writing
//! one marks it changed for every view that read it (under a GPUI that retains views, all of them
//! are rebuilt), and keeping times fresh is the windows' own business (`window::tick`).

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{App, TestAppContext};

use crate::app_state::AppState;
use crate::settings::SettingsStore;
use crate::window::test_harness::{draw, drive, isolate_in, start};

/// Count the times `G` is written (or set) from now on.
fn count_writes<G: gpui_kit::Global>(cx: &mut App) -> Rc<Cell<usize>> {
    let writes = Rc::new(Cell::new(0));
    let counted = writes.clone();
    cx.observe_global::<G>(move |_| counted.set(counted.get() + 1)).detach();
    writes
}

#[gpui_kit::test]
fn an_idle_tick_writes_none_of_the_globals_the_views_read(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "an_idle_tick_writes_none_of_the_globals_the_views_read") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    cx.update(|cx| {
        cx.set_global(crate::status_item::MenuState::new().0);
        crate::fonts::init(cx);
    });
    // The first ticks do what a launch leaves for them (the theme, the menus, a terminal's folder
    // and agents); a quiet one follows.
    for _ in 0..3 {
        cx.update(super::tick::tick);
        drive(cx, &window, |ws, window, cx| ws.tick(window, cx));
    }
    let watched = cx.update(|cx| {
        [
            ("the app state", count_writes::<AppState>(cx)),
            ("the settings", count_writes::<SettingsStore>(cx)),
            ("the updater", count_writes::<crate::update::Updates>(cx)),
            ("the fonts", count_writes::<crate::fonts::LoadedFonts>(cx)),
        ]
    });
    cx.run_until_parked();
    let before: Vec<usize> = watched.iter().map(|(_, writes)| writes.get()).collect();
    // Past every period a tick has (the fonts folder and the projects are looked at every fifth).
    for _ in 0..12 {
        cx.update(super::tick::tick);
        drive(cx, &window, |ws, window, cx| ws.tick(window, cx));
    }
    for ((what, writes), before) in watched.iter().zip(before) {
        assert_eq!(writes.get() - before, 0, "{what} is not written by a tick that changes nothing");
    }
}
