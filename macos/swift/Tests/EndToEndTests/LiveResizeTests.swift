import Foundation
import Testing

/// A real resize of the window while a program runs (T18): the window's corner dragged as a hand drags it (the press lands on the
/// window frame, whose own loop reads the queued moves: AppKit's live resize, `macos/parity/evidence/s-tooling/T18.txt`), with a
/// shell printing numbers the whole time. What must hold afterwards: the program was told the grid the view drew (`stty size`), no
/// line of the output is missing or twice in the terminal's text, a long line is one line however it was wrapped on the way, and
/// the cursor is where the prompt is.
extension EndToEnd {
    /// The terminal's text as the core holds it, history included, as lines with the rows a long line wrapped over joined
    /// (`wh read --lines --join`): what a program that reads the screen would see.
    @MainActor private static func text(_ app: Instance, terminal: Int, history: Int) async throws -> [String] {
        let read = try await app.cliRun(["read", "--terminal", "\(terminal)", "--lines", "-\(history):200", "--join"], timeout: 30)
        app.expect(read.status == 0, "wh read: \(String(describing: read.status)) \(read.output.prefix(300))")
        return read.output.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
    }

    @Test("dragging the window's corner while output arrives: the PTY gets the drawn grid, no line is lost or doubled, a long line stays one line, the cursor stays with the prompt")
    @MainActor func resizingWhileOutputArrives() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.until("keys") { $0.contains("focus=terminal") }
            // The terminal's text is read through the automation command line (`wh read`), a feature behind a flag.
            await app.flags(["automation": true])
            await app.send(["agent-script session"])
            guard let start = await Self.grid(app), let terminal = (await app.rows()).selected else { return app.fail("no grid") }
            await Self.ptyTold(app, "before", start)

            // A line longer than the terminal is wide, then a run of numbers, and the drag in the middle of it.
            let wide = String(repeating: "0123456789", count: 14)
            await app.shell("print \(wide)")
            await app.screen { $0.contains(String(wide.prefix(start.columns))) }
            // Ten lines then a pause of a hundredth of a second, 300 times: about three seconds of output, the drag in the first of them.
            await app.shell("zmodload zsh/zselect; for i in {1..300}; do for j in {1..10}; do print line-$((i * 10 + j - 10)); done; zselect -t 1; done; print ended-run")
            // Corner of the window: the frame's bottom right, a little inside it, dragged up and in, in steps, while the lines run.
            let corner = Instance.window
            await app.send(["drag \(corner.width - 2) \(corner.height - 2) \(corner.width - 420) \(corner.height - 260) 24"])
            await app.screen { $0.contains("ended-run") }
            await app.send(["wait-idle 5000"])
            let geometry = await app.reply("geometry")
            guard let after = await Self.grid(app) else { return app.fail("no grid after") }
            app.expect(after.columns < start.columns && after.rows < start.rows, "the drag made the window smaller: \(start) → \(after)\n\(geometry)")
            app.expect(geometry.contains("live=false"), "the drag ended: \(geometry)")
            await Self.ptyTold(app, "after", after)

            // Every number once, in order; the long line whole.
            let lines = try await Self.text(app, terminal: terminal, history: 4000)
            let numbers = lines.compactMap { $0.hasPrefix("line-") ? Int($0.dropFirst(5)) : nil }
            app.expect(numbers == Array(1...3000), "lines 1 to 3000, each once and in order: \(numbers.count) lines, first mismatch at \(zip(numbers, 1...3000).first { $0 != $1 }.map { "\($0)" } ?? "none")")
            app.expect(lines.filter { $0 == wide }.count >= 1, "the long line is whole in one line")

            // The prompt is the last row with text, and the cursor is on it.
            let screen = (await app.reply("screen")).split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
            guard let prompt = screen.lastIndex(where: { !$0.trimmingCharacters(in: .whitespaces).isEmpty }) else { return app.fail("a blank screen") }
            let cursor = await app.reply("cursor")
            app.expect(cursor.contains("at=\(prompt),"), "the cursor is on the prompt's row \(prompt): \(cursor)")

            // And bigger again: the long line unwraps, nothing is lost.
            await app.send(["drag \(corner.width - 420 - 2) \(corner.height - 260 - 2) \(corner.width - 2) \(corner.height - 2) 24", "wait-idle 3000"])
            guard let wider = await Self.grid(app) else { return }
            await Self.ptyTold(app, "again", wider)
            let more = try await Self.text(app, terminal: terminal, history: 4000)
            app.expect(more.compactMap { $0.hasPrefix("line-") ? Int($0.dropFirst(5)) : nil } == Array(1...3000), "still every line once, in order")
        }
    }
}
