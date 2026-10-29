//! Liquid Glass behind the sidebar's cards and the tab bar's tabs (`vibrancy::Glass`): whether the
//! window wants it. The regions, their keys and the probes that tell the glass where they are are the
//! sidebar's (`sidebar::glass`).

use gpui_kit::*;

use crate::settings::store_in;
use crate::window::Workspace;

impl Workspace {
    /// Whether the sidebar's cards, the tab bars and the terminal's surface sit on glass: the
    /// Liquid Glass setting is on, the Mac has it and the system doesn't want windows opaque
    /// (Reduce Transparency, Increase Contrast), whatever the window's own blur and opacity are;
    /// without the blur the window gets a plain fill under the glass (`Blur::set_backing`). Else
    /// everything paints its own surface (the window's opacity and blur keep working as before).
    pub(in crate::window) fn glass_on(&self, cx: &App) -> bool {
        let store = store_in(&self.profile, cx);
        store.settings.glass_on(self.glass.available(), store.system().opaque())
    }
}
