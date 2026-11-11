// The gallery page: every kind of cell the renderer has a path for, in a fixed layout so a
// screenshot of it is a regression check (wide cells, emoji, box drawing, blocks, braille, attributes,
// underline styles, 256 colours and a truecolor ramp).

import Foundation

enum MockGallery {
    static func draw(into screen: inout MockScreen) {
        var row = 0
        func line(_ text: String, flags: CellFlags = [], fg: UInt32 = PackedColor.foreground, bg: UInt32 = PackedColor.background) {
            guard row < screen.rows else { return }
            var column = 0
            for scalar in text.unicodeScalars {
                guard column < screen.cols else { break }
                screen.put(scalar.value, row: row, column: column, fg: fg, bg: bg, flags: flags)
                column += max(1, CellWidth.of(scalar.value))
            }
            row += 1
        }
        /// A line made of several `(text, flags, fg, bg)` parts.
        func parts(_ items: [(String, CellFlags, UInt32, UInt32)]) {
            guard row < screen.rows else { return }
            var column = 0
            for (text, flags, fg, bg) in items {
                for scalar in text.unicodeScalars {
                    guard column < screen.cols else { break }
                    screen.put(scalar.value, row: row, column: column, fg: fg, bg: bg, flags: flags)
                    column += max(1, CellWidth.of(scalar.value))
                }
            }
            row += 1
        }
        let fg = PackedColor.foreground, bg = PackedColor.background
        let red = PackedColor.named(1), green = PackedColor.named(2), blue = PackedColor.named(4)

        line("Wormhole Swift renderer: gallery (mock source)", flags: .bold, fg: PackedColor.named(6))
        parts([
            ("regular ", [], fg, bg), ("bold ", .bold, fg, bg), ("italic ", .italic, fg, bg),
            ("bold italic ", [.bold, .italic], fg, bg), ("dim ", .dim, fg, bg), (" inverse ", .inverse, fg, bg),
            ("hidden", .hidden, fg, bg), (" strike", .strikeout, fg, bg), (" over", .overline, fg, bg),
        ])
        parts([
            ("underline ", .underline, fg, bg), ("double ", .doubleUnderline, red, bg),
            ("curly ", .undercurl, green, bg), ("dotted ", .dottedUnderline, blue, bg), ("dashed", .dashedUnderline, fg, bg),
        ])
        line("ASCII  !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~")
        line("Latin  àáâãäå çèéêë ìíîï ñòóôõö ùúûü ß ÿ  ←↑→↓ ≤≥≠ ∑∏√∞ €£¥")
        line("Wide   日本語のテキスト 漢字かな ｱｲｳｴｵ ハングル 한국어 ＡＢＣ", fg: PackedColor.named(3))
        line("Emoji  😀 🚀 🎉 🔥 🐙 ✨ 🌈", fg: fg)
        line("Box    ┌─┬─┐ ╔═╦═╗ ┏━┳━┓ ╭─╮ ┍┯┑ ╒═╤═╕", fg: PackedColor.named(4))
        line("       │ │ │ ║ ║ ║ ┃ ┃ ┃ │ │ ├─┤ ╞═╪═╡")
        line("       ├─┼─┤ ╠═╬═╣ ┣━╋━┫ ╰─╯ └┴┘ ╘═╧═╛")
        line("       └─┴─┘ ╚═╩═╝ ┗━┻━┛ ╌╌╌ ┄┄┄ ╍╍╍", fg: PackedColor.named(4))
        line("Blocks ▀▄█▌▐ ░▒▓ ▁▂▃▄▅▆▇█ ▏▎▍▌▋▊▉ ▖▗▘▙▚▛▜▝▞▟", fg: PackedColor.named(5))
        line("Braille ⠁⠃⠇⡇⣇⣧⣷⣿ ⢀⣀⣤⣶⣿ ⠉⠛⠿⣿", fg: PackedColor.named(2))
        // 256 colours: the 16 ANSI, then the cube as backgrounds.
        if row < screen.rows {
            var column = 0
            for index in 0..<min(72, screen.cols) {
                screen.put(0x20, row: row, column: column, bg: PackedColor.indexed(index))
                column += 1
            }
            row += 1
        }
        // A truecolor ramp.
        if row < screen.rows {
            for column in 0..<min(72, screen.cols) {
                let t = Double(column) / 71
                screen.put(
                    0x20, row: row, column: column,
                    bg: PackedColor.rgb(UInt8(255 * t), UInt8(255 * (1 - abs(2 * t - 1))), UInt8(255 * (1 - t))))
            }
            row += 1
        }
    }

    /// A one-cell spinner in the gallery's corner: the smallest possible damage, one row.
    static func spinner(_ screen: inout MockScreen, tick: Int) {
        let frames = Array("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏".unicodeScalars)
        let scalar = frames[(tick / 4) % frames.count]
        screen.put(scalar.value, row: 0, column: max(0, screen.cols - 2), fg: PackedColor.named(3))
    }
}
