// A label with what the search found marked (`style::marked`): the characters on the theme's find fill.

import DesignSystem
import SwiftUI

struct MarkedText: View {
    let text: String
    let range: Range<Int>?
    let role: TextRole
    var weight: Font.Weight?
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(attributed).textRole(role, weight: weight)
    }

    private var attributed: AttributedString {
        var string = AttributedString(text)
        guard let range, range.upperBound <= text.count else { return string }
        let start = string.index(string.startIndex, offsetByCharacters: range.lowerBound)
        let end = string.index(string.startIndex, offsetByCharacters: range.upperBound)
        string[start..<end].backgroundColor = tokens.findMatch
        return string
    }
}
