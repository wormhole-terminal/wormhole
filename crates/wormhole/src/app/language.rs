//! The language the interface speaks: chosen once at launch from the system's list of preferred
//! languages, and changed later only by devctl (the Settings row waits for a second real language).
//!
//! The catalog is loaded *when the language is set* (`wormhole_text::set_language`), never from a
//! frame, so this runs before the first window opens. GPUI Kit's own words ("Retry", the calendar's
//! weekdays) follow through `set_locale` in the same call, or a window in another language would show
//! an English widget beside it. The pseudo-language `en-XA` is never negotiated from the system's list;
//! only devctl asks for it by name, so a release build cannot show it.
//!
//! Native menus, the status item and the Dock menu hold their strings in AppKit and are rebuilt by
//! whoever owns them when the language changes at run time; no language but English ships yet, so
//! nothing rebuilds them here. Input placeholders are set when an input is created, so a Settings window
//! that is already open keeps its search and field placeholders in the old language after a devctl
//! language switch (only devctl can switch today); a window opened afterwards has the new one.
//!
//! Part of the app root (`app/`): it may use every surface, and nothing below the app root may call
//! into it.

#[cfg(feature = "devctl")]
use gpui_kit::App;
use wormhole_text::{Language, negotiate, set_language};

/// The shipped language the system's list of preferred languages asks for, English where none of
/// them is shipped (and always in tests, where `platform::locale` knows nothing).
pub(crate) fn from_system() -> Language {
    negotiate(&crate::platform::locale::preferred_languages())
}

/// Sets the language for the first window, before any opens.
pub(crate) fn init() {
    let language = from_system();
    apply(language);
    tracing::info!(language = language.tag(), "interface language");
}

/// Makes `language` the one the interface speaks, and the one GPUI Kit's widgets speak.
fn apply(language: Language) {
    set_language(language);
    gpui_kit::component::set_locale(widget_locale(language));
}

/// Changes the language while the app runs and redraws every window in it.
#[cfg(feature = "devctl")]
pub(crate) fn change(language: Language, cx: &mut App) {
    apply(language);
    cx.refresh_windows();
}

/// The tag GPUI Kit's widgets are given: the pseudo-language is English to them (a tag they lack
/// falls back to English anyway; this keeps that from being by accident).
fn widget_locale(language: Language) -> &'static str {
    if language.is_pseudo() { Language::ENGLISH.tag() } else { language.tag() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_launch_without_a_known_preference_speaks_english() {
        assert_eq!(from_system(), Language::ENGLISH);
    }

    #[test]
    fn the_widgets_speak_english_in_the_pseudo_language() {
        assert_eq!(widget_locale(Language::PSEUDO), "en");
        assert_eq!(widget_locale(Language::ENGLISH), "en");
    }

    #[test]
    fn the_system_never_asks_for_the_pseudo_language() {
        let asked = [Language::PSEUDO.tag(), "en-XA", "en"];
        assert!(!negotiate(&asked).is_pseudo());
    }
}
