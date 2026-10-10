//! What the panel last showed, for devctl's `bounds`: whether it was drawn at all, and the
//! element it describes. GPUI keeps the pick to itself; the panel is where it comes out.

use std::cell::{Cell, RefCell};

use gpui_kit::*;

use super::sources::{element_name, short_file};

/// The element in the panel.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Picked {
    /// In logical pixels, in the window.
    pub(crate) bounds: Bounds<Pixels>,
    /// Where it is built, as `file:line`.
    pub(crate) location: String,
    /// Its id path.
    pub(crate) name: String,
    /// One of GPUI Kit's empty layers, which the panel steps past on its own.
    pub(crate) layer: bool,
}

thread_local! {
    static DRAWN: Cell<bool> = const { Cell::new(false) };
    static PICKED: RefCell<Option<Picked>> = const { RefCell::new(None) };
}

/// Whether the panel was drawn since the last call.
pub(crate) fn take_drawn() -> bool {
    DRAWN.with(|drawn| drawn.replace(false))
}

/// The element the panel showed last.
pub(crate) fn picked() -> Option<Picked> {
    PICKED.with(|picked| picked.borrow().clone())
}

pub(crate) fn forget() {
    PICKED.with(|picked| picked.borrow_mut().take());
}

pub(super) fn drawn() {
    DRAWN.with(|drawn| drawn.set(true));
}

pub(super) fn record(id: &InspectorElementId, state: &DivInspectorState, window: &Window) {
    let location = id.path.source_location;
    let picked = Picked {
        bounds: state.bounds,
        location: format!("{}:{}", short_file(location.file()), location.line()),
        name: element_name(id),
        layer: super::is_kit_layer(id, state, window),
    };
    PICKED.with(|slot| *slot.borrow_mut() = Some(picked));
}
