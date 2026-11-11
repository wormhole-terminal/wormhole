// The words of the sidebar's search, through the words seam (`WordsModel`): each is named like the Fluent message
// in `wormhole-text`'s `swift-window.ftl`. The GPUI sidebar has no search field (its search is the command palette).

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
        Words.text("sidebar-search-count", ["found": .whole(found), "total": .whole(total)], fallback: "\(found) of \(total)")
    }

    static func foundSpoken(_ found: Int, of total: Int) -> String {
        Words.text(
            "sidebar-search-count-spoken", ["found": .whole(found), "total": .whole(total)], fallback: "\(found) of \(total) terminals")
    }
}
