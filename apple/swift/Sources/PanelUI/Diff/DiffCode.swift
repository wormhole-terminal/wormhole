// A line of code as the diff and the viewer draw it: tabs expanded to the tab width (4), then coloured with the theme's
// syntax tokens by the spans the highlighter made for the line. A line without spans, a language without a grammar or a
// theme without a `syntax.*` entry is plain in `text.primary`.

import DesignSystem
import PanelModel
import SwiftUI

struct DiffCode {
    let tokens: ThemeTokens
    static let tabWidth = 4

    func text(_ line: String, spans: [SyntaxSpan]) -> AttributedString {
        let (expanded, columns) = Self.expandTabs(line)
        var out = AttributedString(expanded)
        out.foregroundColor = tokens.textPrimary
        for span in spans {
            guard span.range.upperBound < columns.count, let style = tokens.syntaxStyle(span.key) else { continue }
            let start = out.index(out.startIndex, offsetByCharacters: columns[span.range.lowerBound])
            let end = out.index(out.startIndex, offsetByCharacters: columns[span.range.upperBound])
            out[start..<end].foregroundColor = style.color.color
            if style.italic { out[start..<end].inlinePresentationIntent = .emphasized }
            if style.bold { out[start..<end].inlinePresentationIntent = .stronglyEmphasized }
        }
        return out
    }

    /// The line with its tabs written as spaces to the next tab stop, and where each character of the original starts
    /// in it (one entry more, for the end), which is how the spans' offsets follow the expansion.
    static func expandTabs(_ line: String) -> (text: String, columns: [Int]) {
        var out = "", column = 0
        var columns: [Int] = []
        for ch in line {
            columns.append(column)
            if ch == "\t" {
                let spaces = tabWidth - column % tabWidth
                out += String(repeating: " ", count: spaces)
                column += spaces
            } else {
                out.append(ch)
                column += 1
            }
        }
        columns.append(column)
        return (out, columns)
    }
}
