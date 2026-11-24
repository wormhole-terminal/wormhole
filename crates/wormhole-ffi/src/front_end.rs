//! Tells core that this process is the Swift app, before anything asks a Settings row whether it is shown:
//! a row only the Swift app paints (`one_background`) is hidden from the GPUI app by this fact
//! (`wormhole_core::platform::front_end`).

use wormhole_core::platform::front_end::{self, FrontEnd};

/// Says once, at launch, that this process is the Swift app. The first answer stays, so call it before the
/// first window reads the settings; a second call does nothing.
#[uniffi::export]
pub fn front_end_start() {
    if !front_end::set(FrontEnd::Swift) && !front_end::is_swift() {
        // Something asked first and was told GPUI: the rows only the Swift app shows would stay hidden.
        tracing::error!("front_end_start came after the first read: Swift-only settings rows stay hidden");
    }
}
