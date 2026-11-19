import Foundation
import Testing

/// The window reports and the column switch a program may use once Settings → Terminal lets it act on the window, beside the
/// resize, move, minimise and zoom `windowOperations` covers: the text area in pixels (CSI 14 t), the screen in characters
/// (CSI 19 t), the window's state (CSI 11 t), and DECCOLM's 132 and 80 columns. With `cat`: a reply is echoed on the screen.
extension EndToEnd {
    /// The numbers of the last `CSI <first>;a;b t` reply echoed on the screen.
    private static func report(_ first: Int, in screen: String) -> [Int]? {
        guard let range = screen.range(of: "[\(first);", options: .backwards) else { return nil }
        let numbers = screen[range.upperBound...].prefix { $0.isNumber || $0 == ";" }.split(separator: ";").compactMap { Int($0) }
        return numbers.count == 2 ? numbers : nil
    }

    @Test("with window operations allowed a program reads the text area in pixels, the screen in characters and the window's state, and switches the window's columns with DECCOLM")
    @MainActor func windowReportsAndColumns() async throws {
        try await Instance.with { app in
            let e = "\u{1b}"
            await app.send(["set allow_window_ops=true"])
            await app.until("get allow_window_ops") { $0.contains("true") }
            // A terminal takes the switch when it starts. Asked until it answers: it reads the setting a moment after the file changed.
            var said = ["", ""]
            var opened = 1
            let start = Date()
            repeat {
                await app.send(["key cmd-t"])
                opened += 1
                await app.until("rows") { Listing($0).live.count == opened }
                await app.send(["wait-idle 1500"])
                await app.emit("\(e)[14t")
                said = Array((await app.send(["wait-idle 1500", "screen", "state"])).suffix(2))
                await app.discardLine()
            } while Self.report(4, in: said[0]) == nil && Date().timeIntervalSince(start) < 15
            guard let pixels = Self.report(4, in: said[0]) else { return app.fail("CSI 14 t is answered:\n\(said[0])") }
            // The text area as the engine counts it: its grid times the cell in whole points (height, then width).
            let grid = (Instance.value("grid", in: said[1]) ?? "").split(separator: "x").compactMap { Int($0) }
            let cell = (Instance.value("cell", in: said[1]) ?? "").replacingOccurrences(of: "px", with: "").split(separator: "x").compactMap { Double($0) }
            let scale = Instance.value("scale", in: said[1]).flatMap { Double($0) } ?? 0
            guard grid.count == 2, cell.count == 2, scale > 0 else { return app.fail("the grid and the cell: \(said[1])") }
            app.expect(pixels == [grid[1] * Int(cell[1] / scale), grid[0] * Int(cell[0] / scale)], "the text area of \(grid[0])x\(grid[1]) cells of \(cell[0] / scale)x\(cell[1] / scale) points: reported \(pixels)")

            // The screen in characters is at least the terminal's own grid.
            await app.clearScreen()
            await app.discardLine()
            await app.emit("\(e)[19t")
            let most = Self.report(9, in: await app.screen { Self.report(9, in: $0) != nil })
            await app.discardLine()
            app.expect((most?[0] ?? 0) >= grid[1] && (most?[1] ?? 0) >= grid[0], "the most cells the screen holds: \(String(describing: most)) for a grid of \(grid)")

            // Not minimised: CSI 11 t answers 1.
            await app.clearScreen()
            await app.discardLine()
            await app.emit("\(e)[11t")
            await app.screen { $0.contains("^[[1t") }
            await app.discardLine()

            // DECCOLM (allowed by mode 40): 132 columns as far as the display has room for the window (more than before),
            // then 80, which fits anywhere.
            await app.emit("\(e)[?40h\(e)[?3h")
            await app.until("state") { (Instance.value("grid", in: $0)?.split(separator: "x").first.flatMap { Int($0) } ?? 0) > grid[0] }
            await app.discardLine()
            await app.emit("\(e)[?3l")
            await app.until("state") { Instance.value("grid", in: $0)?.hasPrefix("80x") == true }
        }
    }
}
