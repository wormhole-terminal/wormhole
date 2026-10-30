//! Tracks AppKit pointer actions, which GPUI does not see as mouse input.
use gpui_kit::{Window, WindowId};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default)]
struct WindowState {
    hosts: usize,
    pointer: bool,
}

thread_local! {
    static WINDOWS: RefCell<HashMap<WindowId, WindowState>> = RefCell::new(HashMap::new());
}

pub(crate) fn register(id: WindowId) {
    WINDOWS.with(|windows| windows.borrow_mut().entry(id).or_default().hosts += 1);
}

pub(crate) fn pointer(window: &Window) {
    WINDOWS.with(|windows| {
        if let Some(state) = windows.borrow_mut().get_mut(&window.window_handle().window_id()) {
            state.pointer = true;
        }
    });
}

pub(crate) fn keyboard(window: &mut Window) {
    if WINDOWS.with(|windows| {
        windows
            .borrow_mut()
            .get_mut(&window.window_handle().window_id())
            .is_some_and(|state| std::mem::take(&mut state.pointer))
    }) {
        window.refresh();
    }
}

pub(crate) fn visible(window: &Window) -> bool {
    window.last_input_was_keyboard()
        && !WINDOWS.with(|windows| {
            windows.borrow().get(&window.window_handle().window_id()).is_some_and(|state| state.pointer)
        })
}

pub(crate) fn forget(id: WindowId) {
    WINDOWS.with(|windows| {
        let mut windows = windows.borrow_mut();
        if let Some(state) = windows.get_mut(&id) {
            state.hosts -= 1;
            if state.hosts == 0 {
                windows.remove(&id);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_state_survives_one_host_closing_and_is_scoped_to_its_window() {
        let first = WindowId::from(0x100000001);
        let second = WindowId::from(0x100000002);
        register(first);
        register(first);
        register(second);
        WINDOWS.with(|windows| windows.borrow_mut().get_mut(&first).unwrap().pointer = true);
        forget(first);
        WINDOWS.with(|windows| {
            let windows = windows.borrow();
            assert!(windows[&first].pointer);
            assert!(!windows[&second].pointer);
        });
        forget(first);
        forget(second);
        WINDOWS.with(|windows| assert!(windows.borrow().is_empty()));
    }
}
