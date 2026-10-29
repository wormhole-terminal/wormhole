//! What the system says of itself.

#[cfg(target_os = "linux")]
use super::linux::system as os;
#[cfg(target_os = "macos")]
use super::macos::system as os;

/// The running macOS version as the system writes it ("15.0.1"), or `None` elsewhere or if it can't be
/// determined.
///
/// Read from the system's version file, not by running `sw_vers`: this is called while the app
/// starts, on the UI thread, and a process spawn (fork, exec, dyld) costs milliseconds there for
/// one short string. `sw_vers` remains the fallback if the file can't be read.
pub fn macos_version_text() -> Option<String> {
    os::version_text()
}

/// Whether this is macOS 26 or later, the first with Liquid Glass. Read once: Settings asks on every
/// frame, and the version doesn't change while the app runs.
pub fn has_liquid_glass() -> bool {
    static HAS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *HAS.get_or_init(|| macos_version_text().as_deref().is_some_and(major_has_liquid_glass))
}

fn major_has_liquid_glass(version: &str) -> bool {
    version.split('.').next().and_then(|major| major.parse::<u64>().ok()).is_some_and(|major| major >= 26)
}

#[cfg(test)]
mod tests {
    use super::major_has_liquid_glass;

    #[test]
    fn liquid_glass_starts_with_macos_26() {
        assert!(!major_has_liquid_glass("15.6.1"));
        assert!(major_has_liquid_glass("26.0"));
        assert!(major_has_liquid_glass("27.2"));
        assert!(!major_has_liquid_glass(""), "an unreadable version has no glass");
    }
}
