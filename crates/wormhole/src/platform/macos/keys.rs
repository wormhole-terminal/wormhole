//! AppKit's side of `platform::keys`: the current event's device-dependent modifier flags.

use objc2_app_kit::NSEvent;

#[inline]
pub fn option_sides() -> (bool, bool) {
    // NX_DEVICELALTKEYMASK and NX_DEVICERALTKEYMASK (IOKit's IOLLEvent.h).
    const LEFT_OPTION: usize = 0x20;
    const RIGHT_OPTION: usize = 0x40;
    let flags = NSEvent::modifierFlags_class().0;
    (flags & LEFT_OPTION != 0, flags & RIGHT_OPTION != 0)
}
