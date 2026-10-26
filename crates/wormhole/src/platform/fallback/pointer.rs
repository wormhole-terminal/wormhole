//! `platform::pointer` where there is nothing to ask.

use crate::platform::pointer::Pointer;

pub fn on_screen() -> Option<Pointer> {
    None
}
