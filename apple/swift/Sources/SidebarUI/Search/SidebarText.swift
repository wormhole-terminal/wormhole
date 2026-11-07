// The words of the sidebar's search, through the words seam (`WordsModel`): each is named like the Fluent message
// it would be in `wormhole-text`. The GPUI sidebar has no search field (its search is the command palette), so
// these have no catalog entry yet and read as the English until one is added.

import Foundation
import WordsModel

enum SidebarText {
    static var searchPlaceholder: String { Words.text("sidebar-search-placeholder", fallback: "Search terminals") }
    static var searchClear: String { Words.text("sidebar-search-clear", fallback: "Clear search") }
    static var searchNothing: String { Words.text("sidebar-search-nothing", fallback: "No terminal matches") }
    static var searchHint: String {
        Words.text("sidebar-search-hint", fallback: "Return opens the first one, Escape ends the search")
    }

    /// The count at the field's end, and the same for VoiceOver.
    static func found(_ found: Int, of total: Int) -> String {
        Words.text("sidebar-search-count", fallback: "\(found) of \(total)")
    }

    static func foundSpoken(_ found: Int, of total: Int) -> String {
        Words.text("sidebar-search-count-spoken", fallback: "\(found) of \(total) terminals")
    }
}
