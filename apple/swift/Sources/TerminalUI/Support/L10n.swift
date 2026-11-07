// The words of this module. One function, so a catalog can take over: keys are named like the Fluent ids of
// `crates/wormhole-text/locales/en/*.ftl` (terminal-*, notice-*, hosts-*), the English is the default value.
// Until the FFI hands over the real table, every sentence a person reads is written once, in an `L10n+Area.swift`.

import Foundation

enum L10n {
    static func tr(_ key: StaticString, _ english: String.LocalizationValue) -> String {
        String(localized: key, defaultValue: english)
    }
}
