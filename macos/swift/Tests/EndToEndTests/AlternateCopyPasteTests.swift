import Foundation
import Testing

/// The terminal's second pair of clipboard keys: ⇧⌘V pastes and ⌥⌘C copies, as ⌘V and ⌘C do (TO-VERIFY line 18). Read from
/// the screen (what reached the program) and the instance's own clipboard.
extension EndToEnd {
    @Test("⇧⌘V pastes into the terminal and ⌥⌘C copies its selection")
    @MainActor func alternateCopyAndPasteKeys() async throws {
        try await Instance.with { app in
            await app.send(["pasteboard-set pasted-by-shift-cmd-v", "key shift-cmd-v"])
            await app.screen(showing: "pasted-by-shift-cmd-v")
            await app.send(["type \\n"])
            let screen = await app.screen { $0.components(separatedBy: "pasted-by-shift-cmd-v").count == 3 }
            let row = String(screen.split(separator: "\n", omittingEmptySubsequences: false).first ?? "")
            await app.send(["pasteboard-set nothing-yet", "select-cells 0 0 0 5", "frames 2", "key alt-cmd-c"])
            let copied = await app.pasteboard()
            app.expect(copied == String(row.prefix(6)), "⌥⌘C copied the selected cells: \"\(copied)\" of \"\(row)\"")
        }
    }
}
