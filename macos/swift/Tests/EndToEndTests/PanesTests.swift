import Foundation
import Testing

extension EndToEnd {
    @Test("splits right and down, focus moves between panes, a pane closes")
    @MainActor func splits() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-d"])
            let right = Panes(await app.until("panes") { Panes($0).ids.count == 2 })
            app.expect(right.isSplit, "two panes make a split")
            await app.send(["key shift-cmd-d"])
            let three = Panes(await app.until("panes") { Panes($0).ids.count == 3 })
            // The pane that was made last has the keys; the one to its left takes them by ⌥⌘←, and gives them back by ⌥⌘→.
            let last = three.focused
            await app.send(["key alt-cmd-left"])
            let left = Panes(await app.until("panes") { Panes($0).focused != last })
            app.expect(left.focused != nil, "a pane has the focus")
            await app.send(["key alt-cmd-right"])
            await app.until("panes") { Panes($0).focused != left.focused }
            await app.closeSelected()
            let two = Panes(await app.until("panes") { Panes($0).ids.count == 2 })
            app.expect(two.isSplit, "the two that are left are still a split")
        }
    }
}
