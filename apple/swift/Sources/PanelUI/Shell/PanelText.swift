// The panel's words. GPUI's live in `wormhole-text`'s Fluent catalogs; until the FFI brings that table, each message is
// one call here, keyed like the Fluent id (`files-menu-pin`) with the English as its default, so a string catalog or the
// real table replaces the body and no view changes.

import Foundation
import WordsModel

func panelText(_ key: StaticString, _ english: String.LocalizationValue) -> String {
    Words.text(key, fallback: String(localized: key, defaultValue: english))
}

/// The same for a message with variables, named as its Fluent text names them.
func panelText(_ key: StaticString, _ args: [String: WordValue], _ english: String.LocalizationValue) -> String {
    Words.text(key, args, fallback: String(localized: key, defaultValue: english))
}
