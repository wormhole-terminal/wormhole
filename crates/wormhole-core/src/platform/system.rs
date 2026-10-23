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
