import CoreGraphics
import Foundation
import Testing

/// What the renderer draws for text that is not one narrow letter per cell (T02): wide characters, colour emoji, ZWJ and flag
/// sequences, combining marks, a character only a fallback font has, and the ligatures of the font as the setting says. The
/// terminal's own picture (`shot`) read cell by cell, against the cells the engine gave each sample (a magenta space after each one: the
/// column it lands in is the sample's width). `cat` writes the samples back; each is placed with CUP on a row of its own.
extension EndToEnd {
    /// The columns of a sample's row that hold ink, as ranges: where the picture is, against where the engine put its cells.
    private struct Sample {
        let width: Int
        let inked: [Int]
        let coloured: Bool
        /// How many pixels of each colour the sample's cells have.
        let picture: [RGB: Int]
    }

    @MainActor private static func samples(_ app: Instance, _ texts: [String], background: RGB, name: String) async -> [Sample]? {
        await app.clearScreen()
        for (row, text) in texts.enumerated() { await app.emit("\u{1b}[\(row + 1);1H\(text)\u{1b}[48;2;255;0;255m \u{1b}[0m") }
        await app.send(["wait-idle 2000"])
        guard let shot = await app.shot(name) else { return nil }
        let marker = RGB(255, 0, 255)
        return texts.indices.map { row in
            // The cell after the sample is a magenta space: the column it is in is how many cells the engine gave the sample.
            let bar = (0..<40).first { shot.pixel(row: row, column: $0) == marker } ?? 0
            var inked: [Int] = [], coloured = false
            for column in 0..<bar {
                let marks = shot.histogram(row: row, column: column).filter { $0.key != background }
                if !marks.isEmpty { inked.append(column) }
                if marks.keys.contains(where: { max(Int($0.red), Int($0.green), Int($0.blue)) - min(Int($0.red), Int($0.green), Int($0.blue)) > 40 }) { coloured = true }
            }
            return Sample(width: bar, inked: inked, coloured: coloured, picture: shot.histogram(row: row, column: 0, columns: bar))
        }
    }

    @Test("wide characters, emoji, ZWJ and flag sequences, combining marks and a fallback font's character are drawn in the cells the engine gave them")
    @MainActor func drawnShaping() async throws {
        try await Instance.with { app in
            await app.send(["opacity 1"])
            let look = Readout(await app.reply("window-look")).first("window-look")
            guard let background = look?["terminal"].flatMap({ RGB(hex: $0) }) else { return app.fail("no background") }
            let texts = [
                "日本語", "한글", "😀", "👨\u{200D}👩\u{200D}👧", "🇩🇪", "👍🏽", "e\u{301}", "e\u{301}\u{302}", "ก", "नमस्ते", "abc", "👨", "e",
            ]
            guard let found = await Self.samples(app, texts, background: background, name: "shaping") else { return }
            let widths = found.map(\.width)
            app.expect(widths[0] == 6 && found[0].inked.count == 6, "three wide characters: six cells, ink in each: \(widths[0]) \(found[0].inked)")
            app.expect(widths[1] == 4 && found[1].inked.count >= 3, "two Hangul syllables: four cells: \(widths[1]) \(found[1].inked)")
            app.expect(widths[2] == 2 && found[2].coloured && Set(found[2].inked) == [0, 1], "an emoji is two cells of colour: \(widths[2]) \(found[2].inked) \(found[2].coloured)")
            // A sequence the engine makes one cluster is drawn as one picture inside its cells; one it does not is drawn as the parts
            // it counts. Either way the ink is exactly the cells the engine reserved, in colour.
            for (index, name) in [(3, "ZWJ family"), (4, "flag"), (5, "skin tone")] {
                app.expect(found[index].coloured, "\(name) is drawn in colour")
                app.expect(Set(found[index].inked) == Set(0..<found[index].width), "\(name): ink in each of its \(found[index].width) cells and none beside them: \(found[index].inked)")
            }
            app.expect(widths[3] == 2 && widths[4] == 2 && widths[5] == 2, "one cluster, two cells: family \(widths[3]), flag \(widths[4]), skin tone \(widths[5])")
            app.expect(widths[6] == 1 && widths[7] == 1, "a combining mark takes no cell of its own: \(widths[6]), \(widths[7])")
            app.expect(found[8].width == 1 && found[8].inked == [0], "a character only a fallback font has is drawn: \(found[8].inked)")
            app.expect(widths[9] >= 3 && !found[9].inked.isEmpty, "Devanagari is drawn: \(widths[9]) cells")
            app.expect(widths[10] == 3, "plain text one cell each")
            app.expect(found[3].picture != found[11].picture, "the family is a picture of its own, not the first emoji alone")
            app.expect(found[6].picture != found[12].picture, "the accent is drawn: e with a combining acute is not a plain e")
        }
    }

    @Test("ligatures: the font forms => -> != == as one picture while the setting is on, and draws the characters apart when it is off")
    @MainActor func ligaturesFollowTheSetting() async throws {
        try await Instance.with { app in
            await app.send(["opacity 1"])
            let look = Readout(await app.reply("window-look")).first("window-look")
            guard let background = look?["terminal"].flatMap({ RGB(hex: $0) }) else { return app.fail("no background") }
            let fonts = await app.reply("fonts")
            app.expect(fonts, contains: "ligatures=true")
            let texts = ["->", "=>", "!=", "==", "abc"]
            func cells(_ shot: GridShot) -> [[RGB: Int]] {
                texts.indices.map { row in shot.histogram(row: row, column: 0, columns: texts[row].count) }
            }
            await app.clearScreen()
            for (row, text) in texts.enumerated() { await app.emit("\u{1b}[\(row + 1);1H\(text)") }
            await app.send(["wait-idle 2000"])
            guard let on = await app.shot("ligatures-on") else { return }
            let withLigatures = cells(on)
            await app.send(["set font_ligatures=false"])
            await app.until("fonts") { $0.contains("ligatures=false") }
            await app.send(["wait-idle 2000"])
            guard let off = await app.shot("ligatures-off") else { return }
            let without = cells(off)
            for row in 0..<4 { app.expect(withLigatures[row] != without[row], "\(texts[row]) is drawn differently with ligatures on and off") }
            app.expect(withLigatures[4] == without[4], "text with no ligature is drawn the same")
            app.expect(withLigatures.allSatisfy { $0.contains { $0.key != background } }, "every sample is drawn")
        }
    }
}
