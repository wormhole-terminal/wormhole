// A line of code as the diff draws it: tabs expanded to the tab width (4), then coloured with the theme's syntax
// tokens. A language without a grammar here, or a theme without a `syntax.*` entry, leaves the text plain in
// `text.primary`.

import DesignSystem
import PanelModel
import SwiftUI

struct DiffCode {
    let tokens: ThemeTokens
    let language: SyntaxLanguage?
    static let tabWidth = 4

    func text(_ line: String) -> AttributedString {
        let expanded = Self.expandTabs(line)
        var out = AttributedString(expanded)
        out.foregroundColor = tokens.textPrimary
        guard let language else { return out }
        for span in SyntaxLexer.spans(in: expanded, language: language) {
            guard let style = tokens.syntaxStyle(span.key) else { continue }
            let start = out.index(out.startIndex, offsetByCharacters: span.range.lowerBound)
            let end = out.index(start, offsetByCharacters: span.range.count)
            out[start..<end].foregroundColor = style.color.color
            if style.italic { out[start..<end].inlinePresentationIntent = .emphasized }
            if style.bold { out[start..<end].inlinePresentationIntent = .stronglyEmphasized }
        }
        return out
    }

    static func expandTabs(_ line: String) -> String {
        guard line.contains("\t") else { return line }
        var out = "", column = 0
        for ch in line {
            if ch == "\t" {
                let spaces = tabWidth - column % tabWidth
                out += String(repeating: " ", count: spaces)
                column += spaces
            } else {
                out.append(ch)
                column += 1
            }
        }
        return out
    }
}
