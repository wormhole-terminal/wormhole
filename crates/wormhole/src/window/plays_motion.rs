//! Whether the window plays the motions the design fixed (the panel's tab switch, the palette's exit, a split
//! opening, the status bar's counts): the Motion feature is on in this build and the system does not ask for
//! Reduce Motion. The column styles and the list's motions read their choice instead (`motion_in_force`), which
//! is Classic without the feature, and GPUI's own animations hold still under Reduce Motion by themselves; this
//! is for what a window starts and ends by hand.

use gpui_kit::*;

use crate::settings::settings_in;
use crate::window::Workspace;

impl Workspace {
    /// Whether a motion that is begun now is played, and not cut.
    pub(in crate::window) fn plays_motion(&self, cx: &App) -> bool {
        settings_in(&self.profile, cx).motion_feature_on() && !cx.reduce_motion()
    }
}
