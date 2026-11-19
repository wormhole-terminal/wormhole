import Foundation
import Testing

/// Text zoom out and the grid the program is told (T17), and a window that grows (T18): read with `fonts` (the view's point
/// size), `state` (the grid the view lays out) and what `stty size` prints in a real shell (the size the PTY was given).
extension EndToEnd {
    /// The selected view's grid from `state` (`grid=100x40`): columns and rows.
    @MainActor static func grid(_ app: Instance) async -> (columns: Int, rows: Int)? {
        let sides = (Instance.value("grid", in: await app.reply("state")) ?? "").split(separator: "x").compactMap { Int($0) }
        return sides.count == 2 ? (sides[0], sides[1]) : nil
    }

    /// `stty size` at the shell prints the PTY's rows and columns: they are the view's grid. `tag` tells this answer from an
    /// earlier one still on the screen (the typed line shows the command, not its answer).
    @MainActor static func ptyTold(_ app: Instance, _ tag: String, _ grid: (columns: Int, rows: Int)) async {
        await app.shell("echo \(tag)=$(stty size | tr ' ' x)")
        await app.screen(showing: "\(tag)=\(grid.rows)x\(grid.columns)")
    }

    /// The selected view's point size (`fonts`' `view=13.0pt`).
    @MainActor static func points(_ app: Instance) async -> Double? {
        Instance.value("view", in: await app.reply("fonts")).flatMap { Double($0.replacingOccurrences(of: "pt", with: "")) }
    }

    // GPUI: `actions` DecreaseFontSize (the window's zoom one step down), `terminal_view/mod.rs:668` (the PTY resized to the
    // grid the space now makes, after a zoom and after the window grows).
    @Test("⌘- makes the text smaller and the grid larger, and the shell is told; ⌘0 goes back; a taller window gives it more rows")
    @MainActor func zoomOutAndGrowTellThePty() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.until("screen") { $0.contains("%") }
            guard let normal = await Self.grid(app), let size = await Self.points(app) else { return app.fail("no grid or font size") }
            await Self.ptyTold(app, "pty1", normal)

            await app.send(["key cmd--"])
            let smaller = await app.until("fonts") { Instance.value("view", in: $0) != "\(size)pt" }
            let shrunk = Instance.value("view", in: smaller).flatMap { Double($0.replacingOccurrences(of: "pt", with: "")) } ?? size
            app.expect(shrunk < size, "⌘- made the text smaller: \(size) → \(shrunk)")
            await app.until("state") { Instance.value("grid", in: $0) != "\(normal.columns)x\(normal.rows)" }
            guard let zoomed = await Self.grid(app) else { return }
            app.expect(zoomed.columns > normal.columns && zoomed.rows >= normal.rows, "more cells fit: \(normal) → \(zoomed)")
            await Self.ptyTold(app, "pty2", zoomed)

            await app.send(["key cmd-0"])
            await app.until("state") { Instance.value("grid", in: $0) == "\(normal.columns)x\(normal.rows)" }
            app.expect(await Self.points(app) == size, "⌘0 is the setting's size again")

            // The window shorter, then as tall again: the grid loses rows and gets them back, and the shell hears both.
            await app.send(["size \(Instance.window.width)x\(Instance.window.height - 200)"])
            await app.until("state") { Instance.value("grid", in: $0) != "\(normal.columns)x\(normal.rows)" }
            guard let short = await Self.grid(app) else { return }
            app.expect(short.rows < normal.rows, "a shorter window has fewer rows: \(normal) → \(short)")
            await Self.ptyTold(app, "pty3", short)
            await app.send(["size \(Instance.window.width)x\(Instance.window.height)"])
            await app.until("state") { Instance.value("grid", in: $0) == "\(normal.columns)x\(normal.rows)" }
            await Self.ptyTold(app, "pty4", normal)
        }
    }
}
