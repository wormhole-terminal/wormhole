// A result's title with the matched characters marked: accent and semibold, the rest as it is.

import SwiftUI

enum MarkedText {
    /// `ranges` are `Character` offsets (what `PaletteModel` returns), turned into string indices here.
    static func title(_ title: String, ranges: [Range<Int>]) -> AttributedString {
        var text = AttributedString(title)
        let characters = Array(text.characters.indices)
        for range in ranges where range.upperBound <= characters.count {
            let end = range.upperBound == characters.count ? text.endIndex : characters[range.upperBound]
            text[characters[range.lowerBound]..<end].foregroundColor = .accentColor
            text[characters[range.lowerBound]..<end].font = .system(size: 14, weight: .semibold)
        }
        return text
    }
}
