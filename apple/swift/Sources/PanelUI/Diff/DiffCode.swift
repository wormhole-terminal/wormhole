// A line of code as the diff and the viewer draw it: tabs expanded to the tab width (`tab_width`, 4), then coloured with the theme's
// syntax tokens by the spans the highlighter made for the line. A line without spans, a language without a grammar or a
// theme without a `syntax.*` entry is plain in `text.primary`.

import DesignSystem
import PanelModel
import SwiftUI

struct DiffCode {
    let tokens: ThemeTokens
    var tabWidth = 4
    /// The most characters of a line that are drawn (and coloured); the rest is an elision mark (`MAX_ROW_CHARS`).
    static let maxRowCharacters = 5_000

    /// The line drawn: core's rule applied (tabs, placeholders), the spans' colours and the marks' dimming laid on by unicode
    /// scalars, clamped inside the text, so no line a repository holds can put an offset past its end.
    func text(_ line: String, spans: [SyntaxSpan]) -> AttributedString {
        let drawn = DrawnLine(line, spans: spans, tabWidth: tabWidth, limit: Self.maxRowCharacters)
        var out = AttributedString(drawn.text)
        out.foregroundColor = tokens.textPrimary
        func range(_ scalars: Range<Int>) -> Range<AttributedString.Index> {
            out.unicodeScalars.index(out.startIndex, offsetBy: scalars.lowerBound)..<out.unicodeScalars.index(out.startIndex, offsetBy: scalars.upperBound)
        }
        for span in drawn.spans {
            guard let style = tokens.syntaxStyle(span.key) else { continue }
            let at = range(span.range)
            out[at].foregroundColor = style.color.color
            if style.italic { out[at].inlinePresentationIntent = .emphasized }
            if style.bold { out[at].inlinePresentationIntent = .stronglyEmphasized }
        }
        // What a person cannot see, or what would change how the line reads, is a dimmed mark, not code.
        for mark in drawn.marks { out[range(mark)].foregroundColor = tokens.textFaint }
        return out
    }
}
