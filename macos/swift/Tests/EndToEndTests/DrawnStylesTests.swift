import CoreGraphics
import Foundation
import Testing

/// What the renderer draws for a program's text styles (T01): the terminal's own picture (`shot`, read back from its texture)
/// checked cell by cell against what the program asked for. `cat` writes back what is typed, so the terminal reads the
/// sequences as a program's; each sample is placed with CUP on a row of its own, so the typed line's echo (the caret form of
/// the escapes, on the rows between) never reaches a sample's cells. Colours are those the terminal says it has (OSC 4, 10 and 11
/// asked as a program asks them), compared with what is drawn.
extension EndToEnd {
    /// `rgb:RRRR/GGGG/BBBB` as the 8-bit colour the 16-bit answer repeats.
    private static func colour(_ answer: Substring) -> RGB? {
        let parts = answer.split(separator: "/").map { $0.prefix(2) }
        guard parts.count == 3, let red = UInt8(parts[0], radix: 16), let green = UInt8(parts[1], radix: 16), let blue = UInt8(parts[2], radix: 16) else { return nil }
        return RGB(red, green, blue)
    }

    /// The terminal's 16 ANSI colours, its default foreground and background, as it answers a program's OSC 4, 10 and 11.
    @MainActor private static func palette(_ app: Instance) async -> (ansi: [RGB], foreground: RGB, background: RGB)? {
        await app.emit((0..<16).map { "\u{1b}]4;\($0);?\u{07}" }.joined() + "\u{1b}]10;?\u{07}\u{1b}]11;?\u{07}")
        let said = await app.screen { Instance.unwrapped($0).components(separatedBy: "rgb:").count == 19 }
        await app.discardLine()
        let text = Instance.unwrapped(said)
        var ansi = [RGB?](repeating: nil, count: 16), foreground: RGB?, background: RGB?
        for piece in text.components(separatedBy: "^[]").dropFirst() {
            let fields = piece.split(separator: ";", maxSplits: 2).map(String.init)
            if fields.count == 3, fields[0] == "4", let index = Int(fields[1]), index < 16, let at = fields[2].range(of: "rgb:") {
                ansi[index] = colour(fields[2][at.upperBound...].prefix(14))
            } else if fields.count == 2, let at = fields[1].range(of: "rgb:") {
                let value = colour(fields[1][at.upperBound...].prefix(14))
                if fields[0] == "10" { foreground = value } else if fields[0] == "11" { background = value }
            }
        }
        guard let foreground, let background, ansi.allSatisfy({ $0 != nil }) else { return nil }
        return (ansi.compactMap { $0 }, foreground, background)
    }

    /// Writes `text` after the sequence `sgr` on grid row `row` (0-based), then resets. A line of its own for each.
    @MainActor private static func put(_ app: Instance, row: Int, _ sgr: String, _ text: String) async {
        await app.emit("\u{1b}[\(row + 1);1H\u{1b}[\(sgr)m\(text)\u{1b}[0m")
    }

    /// xterm's 256-colour cube and grey ramp, which the engine's table is meant to be (the 16 first are the palette's).
    private static func indexed(_ n: Int) -> RGB {
        if n >= 232 { let level = UInt8(8 + 10 * (n - 232)); return RGB(level, level, level) }
        let level = { (c: Int) -> UInt8 in c == 0 ? 0 : UInt8(55 + 40 * c) }
        let m = n - 16
        return RGB(level(m / 36), level(m / 6 % 6), level(m % 6))
    }

    @Test("colours: the 16, the 256, true colour, defaults, inverse and hidden are drawn as asked, on the text and on the cell")
    @MainActor func drawnColours() async throws {
        try await Instance.with { app in
            // The window's opacity below 1 (the default) lets the blur show through the cleared background: the picture holds the
            // theme's background at that opacity. Drawn opaque, the cells are what the theme's colours are.
            await app.send(["opacity 1"])
            guard let theme = await Self.palette(app) else { return app.fail("the terminal did not answer for its palette:\n\(await app.reply("screen"))") }
            let look = Readout(await app.reply("window-look")).first("window-look")
            app.expect(look?["terminal"].flatMap { RGB(hex: $0) } == theme.background, "the background it says is the one drawn: \(look?["terminal"] ?? "-") against \(theme.background)")
            await app.clearScreen()

            let block = "\u{2588}"
            // Row 0: the 16 as text colours (a full block, so the cell is the colour); row 1: as backgrounds (a space).
            await Self.put(app, row: 0, "", (0..<16).map { "\u{1b}[\($0 < 8 ? 30 + $0 : 82 + $0)m\(block)" }.joined())
            await Self.put(app, row: 1, "", (0..<16).map { "\u{1b}[\($0 < 8 ? 40 + $0 : 92 + $0)m " }.joined())
            // Rows 2 and 3: the 256 as text and as backgrounds, a spread of the cube and the ramp.
            let picks = [16, 21, 46, 82, 196, 201, 226, 231, 232, 244, 255, 17, 100, 178]
            await Self.put(app, row: 2, "", picks.map { "\u{1b}[38;5;\($0)m\(block)" }.joined())
            await Self.put(app, row: 3, "", picks.map { "\u{1b}[48;5;\($0)m " }.joined())
            // Rows 4 to 6: true colour, as semicolons, as a background, and as colons (the form with the empty colour space).
            let truths = [(1, 2, 3), (255, 128, 0), (18, 52, 86), (250, 250, 250), (0, 255, 0)]
            await Self.put(app, row: 4, "", truths.map { "\u{1b}[38;2;\($0.0);\($0.1);\($0.2)m\(block)" }.joined())
            await Self.put(app, row: 5, "", truths.map { "\u{1b}[48;2;\($0.0);\($0.1);\($0.2)m " }.joined())
            await Self.put(app, row: 6, "", truths.map { "\u{1b}[38:2::\($0.0):\($0.1):\($0.2)m\(block)" }.joined())
            // Row 7: the defaults restored after a colour (39, 49, and 0). Row 8: inverse (7): a space shows the text colour, a
            // block the cell's; red text inverted makes the cell red. Row 9: hidden (8) draws no glyph; row 10 is the same word.
            await Self.put(app, row: 7, "31;41", "\u{1b}[39;49m\(block) \u{1b}[31;41m\u{1b}[0m\(block) ")
            await Self.put(app, row: 8, "7", " \u{1b}[31m \u{1b}[0m\u{1b}[7m\(block)")
            await Self.put(app, row: 9, "8", "HHHH")
            await Self.put(app, row: 10, "", "HHHH")
            guard let shot = await app.shot("colours") else { return }
            let slack = 1
            @MainActor func expect(_ got: RGB, _ want: RGB, _ what: String) { app.expect(got.distance(to: want) <= slack, "\(what): drawn \(got), wanted \(want)") }

            for index in 0..<16 {
                expect(shot.background(row: 0, column: index), theme.ansi[index], "SGR \(index < 8 ? 30 + index : 82 + index) text")
                expect(shot.background(row: 1, column: index), theme.ansi[index], "SGR \(index < 8 ? 40 + index : 92 + index) cell")
            }
            for (at, n) in picks.enumerated() {
                expect(shot.background(row: 2, column: at), Self.indexed(n), "38;5;\(n) text")
                expect(shot.background(row: 3, column: at), Self.indexed(n), "48;5;\(n) cell")
            }
            for (at, truth) in truths.enumerated() {
                let want = RGB(UInt8(truth.0), UInt8(truth.1), UInt8(truth.2))
                expect(shot.background(row: 4, column: at), want, "38;2 text")
                expect(shot.background(row: 5, column: at), want, "48;2 cell")
                expect(shot.background(row: 6, column: at), want, "38:2:: text")
            }
            expect(shot.background(row: 7, column: 0), theme.foreground, "39 after a colour: the default text colour")
            expect(shot.background(row: 7, column: 1), theme.background, "49 after a colour: the default background")
            expect(shot.background(row: 7, column: 2), theme.foreground, "0 after both: the default text colour")
            expect(shot.background(row: 8, column: 0), theme.foreground, "inverse space: the text colour as the cell")
            expect(shot.background(row: 8, column: 1), theme.ansi[1], "inverse of red text: a red cell")
            expect(shot.background(row: 8, column: 2), theme.background, "inverse block: the cell's colour as the glyph")
            app.expect(!shot.marks(row: 9, column: 0, over: theme.background, columns: 4), "hidden text draws nothing")
            app.expect(shot.marks(row: 10, column: 0, over: theme.background, columns: 4), "the same text is drawn when not hidden")
        }
    }

    /// The ink of a cell: how many of its pixels are not `background`.
    private static func ink(_ shot: GridShot, row: Int, column: Int, columns: Int = 1, over background: RGB) -> Int {
        shot.histogram(row: row, column: column, columns: columns).filter { $0.key != background }.values.reduce(0, +)
    }

    /// The bands of whole pixel rows a sample's decoration is drawn in: each is the row it starts at and how many rows it takes,
    /// where `part` (0 to 1) of the sample's width is not the background.
    private static func bands(_ shot: GridShot, row: Int, columns: Int, over background: RGB, atLeast part: Double = 0.0) -> [(top: Int, height: Int, inked: [Int])] {
        var found: [(top: Int, height: Int, inked: [Int])] = []
        let wide = shot.width(of: columns)
        for y in 0..<shot.cell.height {
            let inked = shot.inked(row: row, column: 0, line: y, over: background, columns: columns)
            guard Double(inked.count) / Double(wide) > part, !inked.isEmpty else { continue }
            if let last = found.last, last.top + last.height == y { found[found.count - 1].height += 1; found[found.count - 1].inked += inked } else { found.append((y, 1, inked)) }
        }
        return found
    }

    /// The lengths of the runs of ink along pixel row `y` of a sample (the dots and dashes of a dotted or dashed line).
    private static func runs(_ shot: GridShot, row: Int, line: Int, columns: Int, over background: RGB) -> [Int] {
        let inked = Set(shot.inked(row: row, column: 0, line: line, over: background, columns: columns))
        var lengths: [Int] = [], length = 0
        for x in 0..<shot.width(of: columns) {
            if inked.contains(x) { length += 1 } else if length > 0 { lengths.append(length); length = 0 }
        }
        if length > 0 { lengths.append(length) }
        return lengths
    }

    @Test("faces and lines: bold, italic, dim, the five underline styles with their colours, strikethrough, overline, steady blink")
    @MainActor func drawnFacesAndLines() async throws {
        try await Instance.with { app in
            await app.send(["opacity 1"])
            guard let theme = await Self.palette(app) else { return app.fail("no palette") }
            await app.clearScreen()
            // Rows 0 to 3: the same letters plain, bold, italic, bold italic. Row 4: dim next to plain, as full blocks.
            let word = "HHHH"
            await Self.put(app, row: 0, "", word)
            await Self.put(app, row: 1, "1", word)
            await Self.put(app, row: 2, "3", word)
            await Self.put(app, row: 3, "1;3", word)
            await Self.put(app, row: 4, "", "\u{2588}\u{1b}[2m\u{2588}")
            // Rows 6 to 14: eight spaces each with a line under them, in their style; SGR 58 colours the line; the last is plain.
            let lines: [(name: String, sgr: String)] = [
                ("single", "4"), ("double", "4:2"), ("double 21", "21"), ("curly", "4:3"), ("dotted", "4:4"), ("dashed", "4:5"),
                ("coloured", "4;58;2;255;0;0"), ("fg red", "31;4"), ("none", "0"),
            ]
            for (at, line) in lines.enumerated() { await Self.put(app, row: 6 + at, line.sgr, "        ") }
            // Rows 16 to 18: strikethrough, overline, blink (SGR 5) over letters.
            await Self.put(app, row: 16, "9", "        ")
            await Self.put(app, row: 17, "53", "        ")
            await Self.put(app, row: 18, "5", word)
            guard let shot = await app.shot("faces") else { return }
            let cell = shot.cell
            let plain = Self.ink(shot, row: 0, column: 0, columns: 4, over: theme.background)
            app.expect(plain > 0, "plain letters are drawn")

            // Bold is a heavier face: more ink than the same letters plain. Italic leans: the left stem of an H starts further
            // right at its top than at its bottom, where plain letters have it straight.
            let bold = Self.ink(shot, row: 1, column: 0, columns: 4, over: theme.background)
            app.expect(bold > plain, "bold has more ink than plain: \(bold) against \(plain)")
            func lean(_ row: Int) -> Int {
                let top = shot.inked(row: row, column: 0, line: cell.height / 4, over: theme.background, columns: 1).min() ?? 0
                let bottom = shot.inked(row: row, column: 0, line: cell.height * 3 / 4, over: theme.background, columns: 1).min() ?? 0
                return top - bottom
            }
            app.expect(lean(0) == 0, "plain letters stand straight: \(lean(0))")
            app.expect(lean(2) >= 2, "italic leans right: \(lean(2))")
            app.expect(lean(3) >= 2 && Self.ink(shot, row: 3, column: 0, columns: 4, over: theme.background) > Self.ink(shot, row: 2, column: 0, columns: 4, over: theme.background), "bold italic leans and is heavier than italic")

            // Dim fades the text toward the background: a colour between the two, the same share on every channel.
            let full = shot.background(row: 4, column: 0), dim = shot.background(row: 4, column: 1)
            func share(_ dimmed: UInt8, _ text: UInt8, _ ground: UInt8) -> Double? {
                text == ground ? nil : (Double(dimmed) - Double(ground)) / (Double(text) - Double(ground))
            }
            let background = theme.background
            let shares: [Double] = [share(dim.red, full.red, background.red), share(dim.green, full.green, background.green), share(dim.blue, full.blue, background.blue)].compactMap { $0 }
            app.expect(shares.allSatisfy { $0 > 0.2 && $0 < 0.9 } && (shares.max() ?? 0) - (shares.min() ?? 0) < 0.03, "dim is a share of the way from the background to the text: \(shares)")

            // Underlines. Eight cells wide, so a dotted line shows its dots and a curly one its waves.
            let width = 8
            func sample(_ name: String) -> Int { 6 + lines.firstIndex { $0.name == name }! }
            let none = Self.bands(shot, row: sample("none"), columns: width, over: theme.background)
            app.expect(none.isEmpty, "no style, no line: \(none)")
            let single = Self.bands(shot, row: sample("single"), columns: width, over: theme.background)
            app.expect(single.count == 1 && Set(single[0].inked).count == shot.width(of: width), "single: one solid band: \(single.map { ($0.top, $0.height) })")
            let thickness = single.first?.height ?? 1
            for name in ["double", "double 21"] {
                let double = Self.bands(shot, row: sample(name), columns: width, over: theme.background)
                app.expect(double.count == 2 && double.allSatisfy { Set($0.inked).count == shot.width(of: width) }, "\(name): two solid bands: \(double.map { ($0.top, $0.height) })")
            }
            let curly = Self.bands(shot, row: sample("curly"), columns: width, over: theme.background)
            app.expect(curly.count == 1 && curly[0].height > thickness && Set(curly[0].inked).count == shot.width(of: width), "curly: one band taller than a line, every column inked at some height: \(curly.map { ($0.top, $0.height) }) against \(thickness)")
            let curlyRows = (0..<cell.height).map { shot.inked(row: sample("curly"), column: 0, line: $0, over: theme.background, columns: width).count }
            app.expect(curlyRows.filter { $0 > 0 && $0 < shot.width(of: width) }.count >= 2, "curly: rows are inked in parts, the wave: \(curlyRows)")
            let dotted = Self.bands(shot, row: sample("dotted"), columns: width, over: theme.background)
            let dashed = Self.bands(shot, row: sample("dashed"), columns: width, over: theme.background)
            guard let dottedBand = dotted.first, let dashedBand = dashed.first else { return app.fail("dotted or dashed lines are not drawn: \(dotted.count), \(dashed.count)") }
            let dots = Self.runs(shot, row: sample("dotted"), line: dottedBand.top, columns: width, over: theme.background)
            let dashes = Self.runs(shot, row: sample("dashed"), line: dashedBand.top, columns: width, over: theme.background)
            app.expect(dots.count >= 8 && dashes.count >= 4, "dots and dashes have gaps: \(dots) and \(dashes)")
            app.expect((dots.max() ?? 0) < (dashes.max() ?? 0), "a dash is longer than a dot: \(dots) and \(dashes)")

            // The line's colour: the text's own without SGR 58, the one asked for with it.
            func lineColour(_ name: String) -> RGB? {
                let band = Self.bands(shot, row: sample(name), columns: width, over: theme.background).first
                return band.map { shot.pixel(row: sample(name), column: 0, dx: 0, dy: $0.top) }
            }
            app.expect(lineColour("single") == theme.foreground, "an underline is in the text's colour: \(String(describing: lineColour("single")))")
            app.expect(lineColour("fg red") == theme.ansi[1], "and follows it: \(String(describing: lineColour("fg red")))")
            app.expect(lineColour("coloured") == RGB(255, 0, 0), "SGR 58 colours the line apart from the text: \(String(describing: lineColour("coloured")))")

            // Strikethrough runs through the middle of the cell and overline along its top, both solid and as thick as an underline.
            let strike = Self.bands(shot, row: 16, columns: width, over: theme.background)
            app.expect(strike.count == 1 && Set(strike[0].inked).count == shot.width(of: width), "strikethrough: one solid band: \(strike.map { ($0.top, $0.height) })")
            if let band = strike.first {
                let middle = Double(band.top) + Double(band.height) / 2
                app.expect(middle > Double(cell.height) * 0.3 && middle < Double(cell.height) * 0.7, "strikethrough is at the middle: \(middle) of \(cell.height)")
            }
            let over = Self.bands(shot, row: 17, columns: width, over: theme.background)
            app.expect(over.count == 1 && over[0].top < cell.height / 4, "overline: one band at the top: \(over.map { ($0.top, $0.height) })")

            // Blinking text is steady in a window that is not key (the clock runs only for the key window): drawn as plain letters.
            app.expect(Self.ink(shot, row: 18, column: 0, columns: 4, over: theme.background) == plain, "blink: drawn, as the plain letters, while the clock is off")
        }
    }

    @Test("the cursor: a block fills its cell, an underline and a beam are thin lines of the cursor's colour at its bottom and its left")
    @MainActor func drawnCursorShapes() async throws {
        try await Instance.with { app in
            await app.send(["opacity 1"])
            await app.clearScreen()
            // The cell the cursor is in and what is drawn in it, once the program has asked for the shape (DECSCUSR 2, 4, 6: steady).
            @MainActor func drawn(_ shape: Int, face: String, name: String) async -> (shot: GridShot, row: Int, column: Int, colour: RGB)? {
                await app.emit("\u{1b}[\(shape) q")
                await app.send(["wait-idle 2000"])
                let said = await Self.cursor(app) { $0.contains("drawn=\(face)") }
                await app.discardLine()
                guard let place = Instance.value("at", in: said)?.split(separator: ",").compactMap({ Int($0) }), place.count == 2,
                    let colour = Instance.value("colour", in: said).flatMap({ RGB(hex: $0) }), let shot = await app.shot(name)
                else { app.fail("no cursor to look at: \(said)"); return nil }
                // The typed line's echo and `cat`'s answer moved the cursor on: the readout is of the cursor as it is now.
                let now = await Self.cursor(app) { _ in true }
                guard Instance.value("at", in: now) == Instance.value("at", in: said) else { app.fail("the cursor moved between the read-out and the picture: \(said) \(now)"); return nil }
                return (shot, place[0], place[1], colour)
            }
            func line(_ picture: GridShot, _ cell: (row: Int, column: Int), dx: Range<Int>, dy: Int, is colour: RGB) -> Bool {
                dx.allSatisfy { picture.pixel(row: cell.row, column: cell.column, dx: $0, dy: dy) == colour }
            }
            guard let block = await drawn(2, face: "block", name: "cursor-block") else { return }
            let width = block.shot.width(of: 1, from: block.column), height = block.shot.cell.height
            let at = (block.row, block.column)
            app.expect(line(block.shot, at, dx: 0..<width, dy: 0, is: block.colour) && line(block.shot, at, dx: 0..<width, dy: height - 1, is: block.colour), "a block: the cell is the cursor's colour edge to edge")
            guard let underline = await drawn(4, face: "underline", name: "cursor-underline") else { return }
            let under = (underline.row, underline.column)
            app.expect(line(underline.shot, under, dx: 0..<width, dy: height - 1, is: underline.colour) && line(underline.shot, under, dx: 0..<width, dy: height - 3, is: underline.colour), "an underline: the cell's last rows, across it")
            app.expect(!line(underline.shot, under, dx: 0..<width, dy: height / 2, is: underline.colour), "and not the middle of the cell")
            guard let beam = await drawn(6, face: "beam", name: "cursor-beam") else { return }
            let bar = (beam.row, beam.column)
            let column = (0..<height).allSatisfy { beam.shot.pixel(row: bar.0, column: bar.1, dx: 0, dy: $0) == beam.colour }
            app.expect(column && !line(beam.shot, bar, dx: 4..<width, dy: height / 2, is: beam.colour), "a beam: the cell's left edge, its whole height, and thin")
        }
    }
}
