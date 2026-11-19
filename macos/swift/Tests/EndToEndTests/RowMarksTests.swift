import Foundation
import Testing

/// The marks after a row's name, read from the views (`marked`): the lock of a terminal hidden from agents and the fork of a
/// terminal in a worktree; and the tab bar's + with its menu of shells, found where the bar drew it and right-clicked.
extension EndToEnd {
    @Test("Hide from Agents puts the lock after the row's name; showing it again takes the lock away")
    @MainActor func lockMarkFollowsHideFromAgents() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.send(["inspect on"])
            await app.until("marked") { Self.perRow($0, "row.lock") == [0, 0] }
            // The lock: no program that controls Wormhole can see the terminal (sidebar/row/face.rs:200).
            await app.send(["agent-hide 1 on"])
            await app.until("marked") { Self.perRow($0, "row.lock") == [1, 0] }
            await app.send(["agent-hide 1 off"])
            await app.until("marked") { Self.perRow($0, "row.lock") == [0, 0] }
        }
    }

    @Test("a terminal in a worktree of the repository wears the fork after its name; the others do not")
    @MainActor func worktreeMarkOnItsRow() async throws {
        try await Instance.with(shell: .zsh) { app in
            let repo = try app.scratch.repository()
            await app.send(["new \(repo.path)"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.send(["action newTerminalInWorktree"])
            await app.until("windows") { $0.contains("sheet=true") }
            await app.send(["text feature-y", "key return"])
            let opened = Listing(await app.until("rows") { Listing($0).live.count == 3 })
            guard let index = opened.rows.firstIndex(where: { $0.folder.contains("feature-y") }) else { return app.fail("no worktree row:\n\(opened.text)") }
            await app.send(["inspect on"])
            // The fork after the name of a terminal in a worktree (sidebar/row/face.rs:181).
            var wanted = [0, 0, 0]
            wanted[index] = 1
            let marks = await app.until("marked") { Self.perRow($0, "row.worktree") == wanted }
            app.expect(Self.perRow(marks, "row.worktree") == wanted, "the fork on the worktree's row only:\n\(marks)")
        }
    }

    @Test("the tab bar has its + after the tabs, and a shell picked from its menu opens a terminal")
    @MainActor func tabBarPlusOffersTheShells() async throws {
        try await Instance.with { app in
            // The bar shows once its set has more than one terminal (`TabBarModel.isShown`).
            await app.send(["set terminals_in=\"tabs\"", "key cmd-t"])
            await app.until("tab-bar") { Readout($0).first("tab-bar")?.has("shown") == true }
            let offered = Readout(await app.reply("plus-menu"))
            // A shell by its key (the login shell's line is `-`, which has none to pick by).
            guard let shell = offered.lines.dropFirst().first(where: { $0.kind != "-" })?.kind else { return app.fail("no shell on offer:\n\(offered.text)") }
            await app.send(["inspect on"])
            // The + is the bar's icon button after its tabs, on their line.
            let drawn = Self.frames(await app.until("marked") { Self.frames($0).contains { $0.id == "title.tab" } })
            let tabs = drawn.filter { $0.id == "title.tab" }
            guard let line = tabs.first?.rect,
                let plus = drawn.filter({ $0.id == "icon.button" && abs($0.rect.midY - line.midY) < line.height && $0.rect.minX >= (tabs.map(\.rect.maxX).max() ?? 0) })
                    .min(by: { $0.rect.minX < $1.rect.minX })
            else { return app.fail("no + after the tabs:\n\(drawn)") }
            await app.send(["inspect off"])
            app.expect(plus.rect.width > 0, "the + is drawn after the tabs")
            // The + menu is SwiftUI's own (PanesUI TabBarView: `.contextMenu { newMenu }`), which no AppKit right-click
            // reaches in a test instance: its "New terminal with" choice is picked through `plus-menu`, the menu's own action.
            let before = (await app.rows()).live.count
            app.expect(await app.reply("plus-menu pick \(shell)"), contains: "plus-menu picked \(shell)")
            await app.until("rows") { Listing($0).live.count == before + 1 }
        }
    }
}
