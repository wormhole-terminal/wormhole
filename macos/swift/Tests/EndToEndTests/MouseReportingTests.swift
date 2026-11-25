import CoreGraphics
import Foundation
import Testing

/// The pointer reaching a program through the whole app (T05): clicks, a drag and the wheel made as the window
/// server routes them (`click`, `mouse-down`, `mouse-move`, `mouse-up`, `scroll`; the hover never reaches a view of a window behind others), the program's modes switched by sequences it writes (`cat` writes
/// back what is typed), and what the program reads shown by the line discipline as it echoes it (ESC as `^[`). Every mode and
/// button and modifier has its exact bytes in `MouseModesLiveTests`; this is the route there: window, hit test, cell, report.
extension EndToEnd {
    /// Switches modes on as a program does and waits until the terminal has them (the echo shows the typed line, and the program's
    /// answer is read once the screen settles).
    @MainActor private static func modes(_ app: Instance, _ sequences: String) async {
        await app.emit(sequences)
        await app.send(["wait-idle 2000", "type \u{15}"])
    }

    /// A drag with the press held and the moves sent one by one: `drag` queues the gesture ahead of the press, which SwiftUI's hosting
    /// view around a terminal drains (the terminal gets the press and nothing after it).
    private static func drag(from: (Int, Int), to: (Int, Int), flags: String = "", steps: Int = 8) -> [String] {
        let moves = (1...steps).map { "mouse-move \(from.0 + (to.0 - from.0) * $0 / steps) \(from.1 + (to.1 - from.1) * $0 / steps) \(flags)" }
        return ["mouse-down \(from.0) \(from.1) \(flags)"] + moves + ["mouse-up \(to.0) \(to.1) \(flags)"]
    }

    /// The screen's rows as one text, so a report the line wrapped is still found.
    @MainActor private static func echoed(_ app: Instance, until done: (String) -> Bool) async -> String {
        Instance.unwrapped(await app.screen { done(Instance.unwrapped($0)) })
    }

    @Test("a click, a drag and the wheel reach a program as the reports it asked for; Shift gives the pointer back to the selection")
    @MainActor func pointerReportsThroughTheWholeApp() async throws {
        try await Instance.with { app in
            await app.clearScreen()
            guard let press = await app.cellPoint(row: 10, column: 5), let there = await app.cellPoint(row: 12, column: 9) else { return app.fail("no cells") }
            let at = (Int(press.x), Int(press.y)), to = (Int(there.x), Int(there.y))

            // Normal tracking with SGR: the press and the release of a click, at the cell's 1-based column and row.
            await Self.modes(app, "\u{1b}[?1000h\u{1b}[?1006h")
            await app.send(["click \(at.0) \(at.1)"])
            let click = await Self.echoed(app) { $0.contains("^[[<0;6;11M^[[<0;6;11m") }
            app.expect(click, contains: "^[[<0;6;11M^[[<0;6;11m")
            await app.discardLine()

            // Shift gives the pointer back: no report, and the drag selected the cells it covered (a word the program printed).
            await app.emit("selectable words")
            let printed = await app.screen { $0.contains("selectable words") }
            guard let row = printed.split(separator: "\n", omittingEmptySubsequences: false).firstIndex(where: { $0.hasPrefix("selectable words") }),
                let start = await app.cellPoint(row: row, column: 0), let end = await app.cellPoint(row: row, column: 9)
            else { return app.fail("the printed word is not on the screen:\n\(printed)") }
            await app.discardLine()
            let before = Instance.unwrapped(await app.reply("screen")).components(separatedBy: "^[[<").count
            await app.send(Self.drag(from: (Int(start.x) - 3, Int(start.y)), to: (Int(end.x) + 3, Int(end.y)), flags: "shift") + ["pasteboard-set nothing", "key cmd-c"])
            let copied = await app.pasteboard()
            app.expect(copied == "selectable", "a Shift-drag selected the word: \(copied)")
            app.expect(Instance.unwrapped(await app.reply("screen")).components(separatedBy: "^[[<").count == before, "and the program heard nothing of it")

            // The wheel: up is button 64, down 65, each a press (no release). The position is not checked here: the control folder's
            // wheel event carries none of its own (`MouseModesLiveTests` checks the cell the wheel is turned over).
            await app.send(["scroll \(at.0) \(at.1) -60"])
            let up = await Self.echoed(app) { $0.contains("^[[<64;") }
            app.expect(!up.contains("^[[<65;"), "the wheel turned up reports no down:\n\(up)")
            await app.discardLine()
            await app.send(["scroll \(at.0) \(at.1) 60"])
            let down = await Self.echoed(app) { $0.contains("^[[<65;") }
            app.expect(!down.contains("^[[<64;"), "the wheel turned down reports no up:\n\(down)")
            await app.discardLine()

            // Button-event tracking adds the drag: motion with the button down is 32 + the button, each new cell once.
            await Self.modes(app, "\u{1b}[?1002h")
            await app.send(Self.drag(from: at, to: to))
            let drag = await Self.echoed(app) { $0.contains("^[[<0;10;13m") }
            app.expect(drag, contains: "^[[<0;6;11M")
            app.expect(drag, contains: "^[[<32;")
            app.expect(drag.components(separatedBy: "^[[<32;").count > 2, "the drag reported a motion for more than one cell:\n\(drag)")
            await app.discardLine()

            // Off again: a click is a click on the terminal, not a report.
            await Self.modes(app, "\u{1b}[?1002l\u{1b}[?1000l")
            let quiet = Instance.unwrapped(await app.reply("screen")).components(separatedBy: "^[[<").count
            await app.send(["click \(at.0) \(at.1)"] + Self.drag(from: at, to: to) + ["scroll \(at.0) \(at.1) -60", "scroll \(at.0) \(at.1) 60"])
            app.expect(Instance.unwrapped(await app.reply("screen")).components(separatedBy: "^[[<").count == quiet, "a program that stopped asking hears nothing")
        }
    }
}
