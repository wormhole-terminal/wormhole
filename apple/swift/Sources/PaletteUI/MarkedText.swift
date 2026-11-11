// A result's title with the characters the search found marked (`style::marked_text`): the theme's primary,
// semibold; on the selected result the selection's own text colour, still semibold.

import SwiftUI

enum MarkedText {
    /// `ranges` are `Character` offsets (what `PaletteModel` returns), turned into string indices here.
    static func title(_ title: String, ranges: [Range<Int>], mark: Color, size: CGFloat) -> AttributedString {
        var text = AttributedString(title)
        let characters = Array(text.characters.indices)
        for range in ranges where range.lowerBound < range.upperBound && range.upperBound <= characters.count {
            let end = range.upperBound == characters.count ? text.endIndex : characters[range.upperBound]
            text[characters[range.lowerBound]..<end].foregroundColor = mark
            text[characters[range.lowerBound]..<end].font = .system(size: size, weight: .semibold)
        }
        return text
    }
}
