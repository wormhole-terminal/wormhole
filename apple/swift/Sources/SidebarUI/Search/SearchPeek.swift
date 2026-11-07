// When the search field shows by itself, as in Finder and iOS lists: pulled down past the top of the list it comes in,
// and scrolling away from the top takes it out again, unless it holds the keyboard or a query (never hide what is being
// used). ⌘F, VoiceOver's "Search terminals" action and a search in progress show it whatever the scroll says.

import DesignSystem
import CoreGraphics

enum SearchPeek {
    /// How far past the top the list is pulled before the field comes in: a deliberate pull, not the bounce of a flick.
    static let pull: CGFloat = Space._4
    /// How far from the top the list is scrolled before an idle field goes: a little, so the bounce coming back to rest
    /// does not hide what the pull just showed.
    static let away: CGFloat = Space._2

    /// Whether the field shows after the list moved to `offset` (0 at the top, negative while pulled past it).
    static func shown(_ shown: Bool, offset: CGFloat, inUse: Bool) -> Bool {
        if offset <= -pull { return true }
        if offset >= away && !inUse { return false }
        return shown
    }
}
