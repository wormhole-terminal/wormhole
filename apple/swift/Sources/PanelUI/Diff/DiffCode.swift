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

    func text(_ line: String, spans: [SyntaxSpan]) -> AttributedString {
        let (expanded, columns, marks) = Self.expandTabs(line, tabWidth: tabWidth)
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
        // What a person cannot see, or what would change how the line reads, is a dimmed mark, not code.
        for mark in marks {
            let start = out.index(out.startIndex, offsetByCharacters: mark.lowerBound)
            let end = out.index(out.startIndex, offsetByCharacters: mark.upperBound)
            out[start..<end].foregroundColor = tokens.textFaint
        }
        return out
    }

    /// How a character that is not drawn as itself is shown (`syntax/visible.rs`): a control is its picture from the
    /// Control Pictures block, one that reorders text (a bidirectional override or isolate, the marks, the line and
    /// paragraph separators) or a C1 control is `<U+202E>`. Without them a file could read as something it is not.
    static func placeholder(_ scalar: Unicode.Scalar) -> String? {
        switch scalar.value {
        case 0x09: nil
        case 0x00...0x1F: String(Unicode.Scalar(0x2400 + scalar.value).map(Character.init) ?? "?")
        case 0x7F: "\u{2421}"
        case 0x80...0x9F, 0x061C, 0x200E, 0x200F, 0x2028, 0x2029, 0x202A...0x202E, 0x2066...0x2069:
            "<U+" + String(format: "%04X", scalar.value) + ">"
        default: nil
        }
    }

    /// The line capped, its tabs written as spaces to the next tab stop and its placeholders spelled out; where each
    /// character of the original starts in it (one entry more, for the end), which is how the spans' offsets follow
    /// the expansion; and where the marks are (placeholders and the elision), in characters of the result.
    static func expandTabs(_ line: String, tabWidth: Int = 4) -> (text: String, columns: [Int], marks: [Range<Int>]) {
        var out = "", column = 0
        var columns: [Int] = []
        var marks: [Range<Int>] = []
        let stop = max(tabWidth, 1)
        for (offset, ch) in line.enumerated() {
            columns.append(column)
            if offset == maxRowCharacters {
                out.append("\u{2026}")
                marks.append(column..<column + 1)
                column += 1
                break
            }
            if ch == "\t" {
                let spaces = stop - column % stop
                out += String(repeating: " ", count: spaces)
                column += spaces
            } else if ch.unicodeScalars.count == 1, let scalar = ch.unicodeScalars.first, let mark = placeholder(scalar) {
                out += mark
                marks.append(column..<column + mark.count)
                column += mark.count
            } else {
                out.append(ch)
                column += 1
            }
        }
        columns.append(column)
        return (out, columns, marks)
    }
}
