//! Small rules of the settings pages that core owns and Swift only asks: what a row says about the state of the
//! window around it.

use wormhole_core::theme::tokens::Translucency;

/// Whether the desktop would show sharp behind the text: translucent without the blur, below the opacity text can be
/// judged at (core's `Translucency::unsafe_without_blur`). The Appearance page warns under the Blur row while it holds.
#[uniffi::export]
pub fn settings_blur_unsafe(opacity: f32, blur: bool) -> bool {
    Translucency { opacity, blur }.unsafe_without_blur()
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
