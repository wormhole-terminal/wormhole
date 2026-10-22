//! Whether this instance is a test instance kept out of the system (`WORMHOLE_BACKGROUND=1`, devctl
//! builds only; what it means for the app is in `crate::background`). The platform's own functions
//! ask it themselves, so the Dock, VoiceOver, the bell's sound and the window order stay untouched
//! without any feature checking.

/// Whether a `WORMHOLE_BACKGROUND` value asks for background mode.
#[cfg_attr(not(feature = "devctl"), allow(dead_code))]
pub fn requested(value: Option<&str>) -> bool {
    value.is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// Whether this instance runs in background mode; never outside devctl builds, so the shipped app
/// can't be put there by its environment.
pub fn on() -> bool {
    #[cfg(feature = "devctl")]
    {
        static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ON.get_or_init(|| requested(std::env::var("WORMHOLE_BACKGROUND").ok().as_deref()))
    }
    #[cfg(not(feature = "devctl"))]
    false
}

#[cfg(test)]
mod tests {
    use super::requested;

    #[test]
    fn only_a_yes_turns_background_mode_on() {
        for yes in ["1", "true", "TRUE", " yes ", "on"] {
            assert!(requested(Some(yes)), "{yes:?}");
        }
        for no in ["", "0", "false", "no", "off", "2", "background"] {
            assert!(!requested(Some(no)), "{no:?}");
        }
        assert!(!requested(None), "unset is the usual app");
    }
}
