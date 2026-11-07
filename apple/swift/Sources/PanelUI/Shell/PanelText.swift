// The panel's words. GPUI's live in `wormhole-text`'s Fluent catalogs; until the FFI brings that table, each message is
// one call here, keyed like the Fluent id (`files-menu-pin`) with the English as its default, so a string catalog or the
// real table replaces the body and no view changes.

import Foundation

func panelText(_ key: StaticString, _ english: String.LocalizationValue) -> String {
    String(localized: key, defaultValue: english)
}
