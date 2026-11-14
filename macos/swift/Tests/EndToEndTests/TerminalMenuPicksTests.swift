import CoreGraphics
import Foundation
import Testing

/// The terminal's context menu, each item picked from the menu a right-click on the grid builds (`right-click` keeps the
/// `NSMenu` unshown, `menu-pick` presses an item as a click does), and what the pick did, read where it lands: the panes,
/// the find bar, the screen, the instance's clipboard.
extension EndToEnd {
    /// A right-click on the cell, the menu's lines. The menu is built for what is under the pointer there.
    @MainActor private static func menu(_ app: Instance, row: Int, column: Int = 2) async -> String {
        guard let cell = await app.cellPoint(row: row, column: column) else {
            app.fail("no place for the terminal's cells in the accessibility tree")
            return ""
        }
        return await app.reply("right-click \(Int(cell.x)) \(Int(cell.y))")
    }

    @Test("picked from the terminal's menu: Select All, Copy, Paste, Find, Copy Command, Copy Output and Put Command at Prompt each act")
    @MainActor func terminalMenuEditPicks() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.markPrompts()
            await app.shell("echo menu-pick-out")
            let listing = await app.until("blocks") { !Instance.blocks($0, running: "echo menu-pick-out").isEmpty }
            guard let block = Instance.blocks(listing, running: "echo menu-pick-out").first, let row = Instance.outputStart(block.line) else {
                return app.fail("echo's block with its output:\n\(listing)")
            }
            // Nothing selected: Copy is off in the menu, and a pick of it copies nothing.
            let first = await Self.menu(app, row: row)
            app.expect(Self.item("Copy", in: first)?.contains("[off]") == true, "Copy is off with nothing selected:\n\(first)")

            // Select All, then Copy from a new menu: the screen's text is on the instance's clipboard.
            await app.send(["menu-pick Select All"])
            let selected = await Self.menu(app, row: row)
            app.expect(Self.item("Copy", in: selected)?.contains("[off]") == false, "Copy is on with a selection:\n\(selected)")
            await app.send(["menu-pick Copy"])
            app.expect(await app.pasteboard(), contains: "menu-pick-out")

            // Copy Output and Copy Command, of the block under the pointer.
            _ = await Self.menu(app, row: row)
            await app.send(["menu-pick Copy Output"])
            app.expect((await app.pasteboard()).trimmingCharacters(in: .whitespacesAndNewlines) == "menu-pick-out", "Copy Output copied the output")
            _ = await Self.menu(app, row: row)
            await app.send(["menu-pick Copy Command"])
            app.expect((await app.pasteboard()).trimmingCharacters(in: .whitespacesAndNewlines) == "echo menu-pick-out", "Copy Command copied the command")

            // Put Command at Prompt: on the prompt's line, not run (no second output line).
            _ = await Self.menu(app, row: row)
            await app.send(["menu-pick Put Command at Prompt"])
            await app.screen { $0.split(separator: "\n").last { !$0.trimmingCharacters(in: .whitespaces).isEmpty }?.hasPrefix("$ echo menu-pick-out") == true }
            app.expect(await app.count("\nmenu-pick-out") == 1, "the command was not run again")
            await app.discardLine()

            // Paste: what the clipboard holds is typed at the prompt.
            await app.send(["pasteboard-set pasted-by-menu"])
            let withPaste = await Self.menu(app, row: row)
            app.expect(Self.item("Paste", in: withPaste)?.contains("[off]") == false, "Paste is on with text to paste:\n\(withPaste)")
            await app.send(["menu-pick Paste"])
            await app.screen { $0.contains("$ pasted-by-menu") }
            await app.discardLine()

            // Find…: the find bar opens over this terminal.
            app.expect(await app.reply("find-state"), contains: "closed")
            _ = await Self.menu(app, row: row)
            await app.send(["menu-pick Find…"])
            await app.until("find-state") { $0.contains(" open") }
        }
    }

    @Test("picked from the terminal's menu: Split Right and Split Down split the terminal clicked, Clear Buffer erases its history")
    @MainActor func terminalMenuSplitAndClearPicks() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.shell("echo before-clear; /usr/bin/seq 1 80")
            await app.screen { $0.contains("80") }
            guard let first = (await app.rows()).live.first else { return app.fail("a terminal") }

            // Clear Buffer: the history goes (the wheel finds nothing above), zsh redraws its prompt at the top.
            _ = await Self.menu(app, row: 3)
            await app.send(["menu-pick Clear Buffer"])
            var cleared = ""
            let start = Date()
            repeat {
                cleared = (await app.send(["scroll 700 400 -6000", "frames 2", "screen", "delay 150"]))[2]
            } while (cleared.contains("before-clear") || cleared.contains("\n40")) && Date().timeIntervalSince(start) < 15
            app.expect(!cleared.contains("before-clear") && !cleared.contains("\n40"), "the history is erased:\n\(cleared)")

            // Split Right: a second pane beside the first, at the same height.
            _ = await Self.menu(app, row: 0)
            await app.send(["menu-pick Split Right"])
            let two = Self.panes(await app.until("panes") { Self.panes($0).count == 2 })
            guard two.count == 2 else { return }
            app.expect(two[0].minY == two[1].minY && two[0].minX != two[1].minX, "the two panes are side by side: \(two)")

            // Split Down, picked over the first terminal (the right-click selects it first): it is the one split, into two
            // panes one above the other in its column.
            guard let frame = await app.terminalFrames().min(by: { $0.minX < $1.minX }) else { return app.fail("the first terminal's frame") }
            _ = await app.reply("right-click \(Int(frame.minX) + 40) \(Int(frame.minY) + 40)")
            app.expect(Listing(await app.reply("rows")).selected == first, "the terminal right-clicked is selected")
            await app.send(["menu-pick Split Down"])
            let three = Self.panes(await app.until("panes") { Self.panes($0).count == 3 })
            guard three.count == 3 else { return }
            let left = three.filter { $0.minX == three.map(\.minX).min() }
            app.expect(left.count == 2 && left[0].minY != left[1].minY, "the left column holds two panes, one above the other: \(three)")
            app.expect((await app.rows()).live.count == 3, "three terminals")
        }
    }

    /// The menu's line for the item titled `title` (`3. Copy [c] [off]`), nil when the menu has none.
    private static func item(_ title: String, in menu: String) -> String? {
        menu.split(separator: "\n").map(String.init).first { line in
            guard let dot = line.range(of: ". ") else { return false }
            let rest = line[dot.upperBound...]
            return rest == title || rest.hasPrefix(title + " [")
        }
    }

    /// The frames of `panes`' pane lines (`pane id x y w h …`).
    private static func panes(_ reply: String) -> [CGRect] {
        reply.split(separator: "\n").filter { $0.hasPrefix("pane ") }.compactMap { line in
            let numbers = line.split(separator: " ").dropFirst(2).prefix(4).compactMap { Double($0) }
            return numbers.count == 4 ? CGRect(x: numbers[0], y: numbers[1], width: numbers[2], height: numbers[3]) : nil
        }
    }
}
