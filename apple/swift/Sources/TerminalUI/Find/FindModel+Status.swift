// What the count in the bar says: "11 of 11", "10,000+", "No results", "Not a valid pattern", or nothing
// before anything is typed and while a first search runs (typing must not flash "No results").

import Foundation

extension FindModel {
    public var countText: String {
        if outcome.invalid { return L10n.findInvalid }
        if text.isEmpty || (isSearching && outcome.count == 0) { return "" }
        if outcome.count == 0 { return L10n.findNoResults }
        let at = (current.map { $0 + 1 } ?? 0).formatted()
        let total = outcome.count.formatted() + (outcome.capped ? "+" : "")
        return L10n.findCount(at, total)
    }

    public var countIsError: Bool { outcome.invalid }
}
