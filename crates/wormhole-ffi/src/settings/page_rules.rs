//! Small rules of the settings pages that core owns and Swift only asks: what a row says about the state of the
//! window around it.

use serde_json::Value;
use wormhole_core::theme::tokens::Translucency;
use wormhole_core::view::layout::title_parts::TitlePart;

use super::handle::SettingsHandle;

/// Whether the desktop would show sharp behind the text: translucent without the blur, below the opacity text can be
/// judged at (core's `Translucency::unsafe_without_blur`). The Appearance page warns under the Blur row while it holds.
#[uniffi::export]
pub fn settings_blur_unsafe(opacity: f32, blur: bool) -> bool {
    Translucency { opacity, blur }.unsafe_without_blur()
}

#[uniffi::export]
impl SettingsHandle {
    /// Shows or hides one part of the title row (`part`: its stored name, `terminal_status`) through core's rule
    /// (`Settings::show_title_part`): a part switched on takes its place among the shown ones. Whether the list
    /// changed; false too for a name that is no part.
    pub fn show_title_part(&self, part: String, shown: bool) -> bool {
        let Ok(part) = serde_json::from_value::<TitlePart>(Value::String(part)) else { return false };
        self.shared.lock().update(|s| s.show_title_part(part, shown))
    }
}

#[cfg(test)]
mod tests {
    use super::settings_blur_unsafe;

    #[test]
    fn the_warning_is_for_a_translucent_window_without_blur() {
        assert!(settings_blur_unsafe(0.8, false));
        assert!(!settings_blur_unsafe(0.8, true));
        assert!(!settings_blur_unsafe(1.0, false));
    }
}
