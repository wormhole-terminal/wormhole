import CoreGraphics
import Foundation
import Testing

/// Fold Output picked from the menu a right-click builds over a block's output in the history (`OutputFoldsTests` folds by
/// `block N fold`, the call the item makes), and Open Link left alone: it would reach the browser.
extension EndToEnd {
    private static let lines = 7201...7212

    @Test("Fold Output picked from the terminal's menu over a block's output in the history folds the block")
    @MainActor func foldOutputFromTheMenu() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.markPrompts()
            await app.flags(["output_folds": true])
            let command = "/usr/bin/seq \(Self.lines.lowerBound) \(Self.lines.upperBound)"
            await app.shell(command)
            await app.shell("/usr/bin/seq 1 160")
            await app.until("blocks") { !Instance.blocks($0, running: "/usr/bin/seq 1 160").isEmpty }
            // The history's top, where the block is; a right-click on one of its lines builds the menu for it.
            var row = -1
            let start = Date()
            repeat {
                let said = await app.send(["scroll 700 400 -6000", "frames 2", "screen"])
                row = said[2].split(separator: "\n", omittingEmptySubsequences: false).firstIndex { $0.trimmingCharacters(in: .whitespaces) == "7205" } ?? -1
            } while row < 0 && Date().timeIntervalSince(start) < 20
            guard row >= 0, let cell = await app.cellPoint(row: row, column: 2) else { return app.fail("the block's line in the history") }
            let menu = await app.reply("right-click \(Int(cell.x)) \(Int(cell.y))")
            app.expect(menu, contains: "Fold Output")
            await app.send(["menu-pick Fold Output"])
            let closed = await app.until("probes") { $0.contains("[closed]") }
            app.expect(closed.contains("lines hidden"), "the block is folded behind a marker that counts its lines: \(closed)")
        }
    }
}
