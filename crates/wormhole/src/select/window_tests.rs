//! The select in a headless GPUI (needs `devctl`'s test support).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui_kit::component::input::InputState;
use gpui_kit::{AppContext as _, Entity, Render, TestAppContext, WindowOptions, div};

use super::{Choice, SearchSelect, SelectTrigger};
use crate::settings::{SettingsStore, ui};

struct Blank;

impl Render for Blank {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        _: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        div()
    }
}

#[gpui_kit::test]
fn a_closed_select_never_builds_its_options(cx: &mut TestAppContext) {
    let search: Rc<RefCell<Option<Entity<InputState>>>> = Rc::default();
    let built = Rc::new(Cell::new(0));
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_global(SettingsStore::for_test());
        let slot = search.clone();
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            *slot.borrow_mut() = Some(cx.new(|cx| InputState::new(window, cx)));
            cx.new(|_| Blank)
        })
        .expect("opens the window");
        let search = search.borrow().clone().expect("the input was made");
        let counter = built.clone();
        let ui = ui(cx);
        let _popover = SearchSelect::new("t", ui, SelectTrigger::new("t-trigger", ui, "label"), &search, cx)
            .options_with(move || {
                counter.set(counter.get() + 1);
                vec![Choice::new(0u8, "one")].into()
            })
            .open(false)
            .into_popover();
    });
    assert_eq!(built.get(), 0, "the options were built for a closed select");
}
