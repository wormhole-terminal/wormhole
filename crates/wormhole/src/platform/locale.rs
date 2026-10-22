//! The languages the user prefers, as the system lists them. The interface's own language is chosen
//! from this list (and the user's setting) by `wormhole-text`; this is only the question to the OS.
//! Tests never ask the real system: there, nothing is known.

use super::os::locale as os;

/// The user's preferred languages, most wanted first, as BCP 47 tags (`["de-DE", "en-US"]`): System
/// Settings → Language & Region, with a per-app choice applied. Empty where it can't be asked (off
/// macOS, in tests), which the caller reads as "no preference", so English.
// Nothing asks yet: the language setting (phase L, step 2) will, and the binary's own copy of this
// module would otherwise warn until then.
#[allow(dead_code)]
pub fn preferred_languages() -> Vec<String> {
    if cfg!(test) { Vec::new() } else { os::preferred_languages() }
}
