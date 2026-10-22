//! AppKit's side of `platform::locale`: `NSLocale`'s ordered list of preferred languages.

use objc2_foundation::NSLocale;

pub fn preferred_languages() -> Vec<String> {
    NSLocale::preferredLanguages().to_vec().into_iter().map(|tag| tag.to_string()).collect()
}
