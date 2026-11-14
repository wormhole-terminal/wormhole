import Foundation
import Testing

/// The three driver gaps the wave-8 lanes found, each now a scenario: ⌘← ⌘→ ⌘⌫ reach the terminal's line editor (TO-VERIFY.md
/// 667), Esc reaches SwiftUI's `onKeyPress` in a field (166: the rename field keeps the old name; the find bar closes), and
/// `a11y` lists SwiftUI's part of the window (a sidebar row's spoken name).
extension EndToEnd {
    @Test("⌘← goes to the line's start, ⌘→ to its end and ⌘⌫ deletes the line, in the shell (667)")
    @MainActor func lineEditorKeys() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.send(["text echo bc"])
            await app.screen(showing: "echo bc")
            // ^A then a letter at the start, ^E then one at the end: `xecho bcz` is no command, and the shell says so.
            await app.send(["key cmd-left", "text x", "key cmd-right", "text z", "key return"])
            await app.screen(showing: "xecho")
            app.expect(await app.count("xecho bcz") >= 1, "the letters went to both ends of the line")
            await app.send(["text never-run-this", "key cmd-backspace", "text echo cleared", "key return"])
            let screen = await app.screen(showing: "cleared")
            app.expect(!screen.contains("never-run-this"), "⌘⌫ took the line away first:\n\(screen)")
        }
    }

    @Test("Esc in a row's name field keeps the old name; Esc in Find closes it (166)")
    @MainActor func escapeInSwiftUIFields() async throws {
        try await Instance.with { app in
            let before = await app.rows()
            guard let row = before.rows.first else { return }
            await app.send(["rename-edit 1", "wait-idle 3000", "text thrown-away", "key esc"])
            let after = await app.rows()
            app.expect(after.row(row.id)?.title == row.title, "Esc kept \(row.title):\n\(after.text)")
            app.expect(!(await app.reply("keys")).contains("NSTextView"), "the field let go of the keys")

            app.expect(await app.reply("key cmd-f"), contains: "+find")
            await app.send(["text abc"])
            let closed = await app.reply("key esc")
            app.expect(!closed.contains("+find"), "Esc in the find field closed Find: \(closed)")
        }
    }

    @Test("a11y lists SwiftUI's part of the window: a sidebar row by its spoken name")
    @MainActor func accessibilityListsSwiftUI() async throws {
        try await Instance.with { app in
            let rows = await app.reply("rows")
            guard let spoken = rows.split(separator: "\n").first(where: { $0.contains("row ") })?.components(separatedBy: "spoken=\"").dropFirst().first?
                .prefix(while: { $0 != "\"" }), !spoken.isEmpty
            else { return app.fail("a row's spoken name:\n\(rows)") }
            let tree = await app.reply("a11y")
            app.expect(tree.contains(String(spoken)), "the row \"\(spoken)\" is in the tree VoiceOver gets:\n\(tree)")
        }
    }
}
