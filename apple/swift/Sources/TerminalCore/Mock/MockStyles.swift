// The page of what the gallery leaves out: double-width and double-height rows, ligature pairs, blinking
// text and underline styles on sized rows. Static, in a fixed layout, so a screenshot of it is a regression
// check next to the GPUI app's rendering of the same escape sequences (DECDWL, DECDHL, SGR 5).

import Foundation

enum MockStyles {
    static func draw(into screen: inout MockScreen) {
        var row = 0
        func line(_ text: String, size: LineSize = .single, flags: CellFlags = [], fg: UInt32 = PackedColor.foreground) {
            guard row < screen.rows else { return }
            screen.setSize(size, row: row)
            var column = 0
            for scalar in text.unicodeScalars {
                guard column < screen.cols else { break }
                screen.put(scalar.value, row: row, column: column, fg: fg, flags: flags)
                column += max(1, CellWidth.of(scalar.value))
            }
            row += 1
        }
        let blue = PackedColor.named(4), green = PackedColor.named(2), red = PackedColor.named(1), yellow = PackedColor.named(3)

        line("Rows: sizes, ligatures, blink (mock source)", flags: .bold, fg: PackedColor.named(6))
        line("Double width 日本 ┌─┐", size: .doubleWidth, fg: blue)
        line("Double height", size: .doubleHeightTop, fg: green)
        line("Double height", size: .doubleHeightBottom, fg: green)
        line("Tall underline", size: .doubleHeightTop, flags: .underline, fg: yellow)
        line("Tall underline", size: .doubleHeightBottom, flags: .underline, fg: yellow)
        line("Wide: strike over curly", size: .doubleWidth, flags: [.strikeout, .overline, .undercurl], fg: red)
        line("Tall box ┌─┬─┐", size: .doubleHeightTop)
        line("Tall box ┌─┬─┐", size: .doubleHeightBottom)
        line("Ligatures  -> => != <= >= == === !== |> <| :: <!-- --> www <-> <=> =/= ::= ## ### /// .. ... && || ++ -- ~> =~")
        line("No ligature across a blank:  - >   = >   ! =")
        line("blinking text, steady text, blinking again", flags: .blink, fg: yellow)
        line("Underlined text on a normal row, for scale", flags: .underline)
    }
}
