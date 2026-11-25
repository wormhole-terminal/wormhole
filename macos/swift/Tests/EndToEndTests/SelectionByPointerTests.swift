import CoreGraphics
import Foundation
import Testing

/// Selection made with the pointer, as a hand makes it (T04): a drag, a double-click on a word, a triple-click on a line,
/// ⌥-drag for a rectangle; what is painted as selected (the renderer's own picture) and what ⌘C puts on the instance's clipboard.
/// A real zsh prints the lines, so the rows hold only what the program wrote, and the typed commands are on rows of their own.
extension EndToEnd {
    /// The row of the screen that holds exactly `text` (the program's output, not the typed command that printed it).
    private static func row(_ screen: String, equalTo text: String) -> Int? {
        screen.split(separator: "\n", omittingEmptySubsequences: false).firstIndex { $0.trimmingCharacters(in: .whitespaces) == text }
    }

    /// A drag as the pointer commands hold one: `drag` queues the rest of the gesture ahead of the press, which SwiftUI's hosting view
    /// around a terminal drains (the terminal gets the press and nothing after it), so the press is held and the moves sent one by one.
    private static func drag(from: CGPoint, to: CGPoint, flags: String = "") -> [String] {
        let steps = 6
        let moves = (1...steps).map { step in
            "mouse-move \(Int(from.x + (to.x - from.x) * Double(step) / Double(steps))) \(Int(from.y + (to.y - from.y) * Double(step) / Double(steps))) \(flags)"
        }
        return ["mouse-down \(Int(from.x)) \(Int(from.y)) \(flags)"] + moves + ["mouse-up \(Int(to.x)) \(Int(to.y)) \(flags)"]
    }

    /// A point in a cell: `at` is the share of the cell's width from its left edge (the left half starts a selection with the cell).
    @MainActor private static func point(_ app: Instance, row: Int, column: Int, at share: Double = 0.5) async -> CGPoint? {
        let state = await app.reply("state")
        guard let centre = await app.cellPoint(row: row, column: column), let cell = Instance.value("cell", in: state)?.replacingOccurrences(of: "px", with: ""),
            let scale = Instance.value("scale", in: state).flatMap({ Double($0) }), let width = cell.split(separator: "x").first.flatMap({ Double($0) })
        else { return nil }
        return CGPoint(x: centre.x + (share - 0.5) * width / scale, y: centre.y)
    }

    @Test("a drag selects the cells between the halves it started and ended in, painted and copied; a double-click a word; a triple-click a line; ⌥-drag a rectangle")
    @MainActor func selectionByThePointer() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.shell(#"print "one two-three four"; print "five six seven"; print "eight nine ten""#)
            let screen = await app.screen { $0.split(separator: "\n").contains { $0.trimmingCharacters(in: .whitespaces) == "eight nine ten" } }
            guard let first = Self.row(screen, equalTo: "one two-three four"), let second = Self.row(screen, equalTo: "five six seven"), let third = Self.row(screen, equalTo: "eight nine ten") else {
                return app.fail("the three lines on the screen:\n\(screen)")
            }

            // A drag from the left half of column 4 to the right half of column 8: the cells 4 to 8, "two-t" at once (the
            // "-" does not end the drag; a drag selects by cell).
            guard let from = await Self.point(app, row: first, column: 4, at: 0.25), let to = await Self.point(app, row: first, column: 8, at: 0.75) else { return app.fail("no cells") }
            await app.send(Self.drag(from: from, to: to) + ["pasteboard-set nothing", "key cmd-c"])
            var copied = await app.pasteboard()
            app.expect(copied == "two-t", "the dragged cells were copied: \(copied)")
            guard let shot = await app.shot("drag") else { return }
            let background = shot.background(row: second, column: 90)
            for column in 0..<20 {
                let selected = (4...8).contains(column)
                let colour = shot.background(row: first, column: column)
                app.expect((colour != background) == selected, "column \(column) is \(selected ? "" : "not ")painted as selected: \(colour)")
            }
            app.expect(Set((4...8).map { shot.background(row: first, column: $0) }).count == 1, "one selection colour")
            app.expect(shot.background(row: second, column: 5) == background, "another row is not selected")

            // A double-click selects the word under it and a triple-click the line (with its line break).
            guard let word = await Self.point(app, row: second, column: 7) else { return }
            await app.send(["double-click \(Int(word.x)) \(Int(word.y))", "key cmd-c"])
            copied = await app.pasteboard()
            app.expect(copied == "six", "double-click: the word: \(copied)")
            await app.send(["triple-click \(Int(word.x)) \(Int(word.y))", "key cmd-c"])
            copied = await app.pasteboard()
            app.expect(copied == "five six seven\\n", "triple-click: the line: \(copied)")
            guard let lineShot = await app.shot("line") else { return }
            app.expect((0..<14).allSatisfy { lineShot.background(row: second, column: $0) != background }, "the line is painted as selected")

            // ⌥-drag makes a rectangle: the first three columns of the three rows, each row's part on a line of its own.
            guard let top = await Self.point(app, row: first, column: 0, at: 0.25), let bottom = await Self.point(app, row: third, column: 2, at: 0.75) else { return }
            await app.send(Self.drag(from: top, to: bottom, flags: "alt") + ["key cmd-c"])
            copied = await app.pasteboard()
            app.expect(copied == "one\\nfiv\\neig", "⌥-drag: a rectangle: \(copied)")
        }
    }
}
