import Foundation
import Testing

extension EndToEnd {
    @Test("a first window opens with one terminal and echoes typed text")
    @MainActor func firstWindow() async throws {
        try await Instance.with { app in
            let first = await app.rows()
            app.expect(first.live.count == 1, "one terminal at first:\n\(first.text)")
            app.expect(first.selected == first.live.first, "the terminal is selected:\n\(first.text)")
            // Typed through the window as a key at a time, and written to the program: `cat` echoes both.
            await app.send(["text hello-wormhole", "type from-the-program\\n"])
            let screen = await app.until("screen") { $0.contains("hello-wormhole") && $0.contains("from-the-program") }
            if !screen.contains("hello-wormhole") { app.fail("the typed keys did not reach the terminal:\n\(await app.keysAndFrame())") }
        }
    }

    @Test("a new terminal is selected by ⌘1 to ⌘9, renamed and closed")
    @MainActor func newSelectRenameClose() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t", "key cmd-t"])
            let three = Listing(await app.until("rows") { Listing($0).live.count == 3 })
            let second = three.rows.first { $0.number == 2 }?.id
            app.expect(second != nil, "a row numbered 2:\n\(three.text)")
            guard let second else { return }
            await app.send(["key cmd-2"])
            await app.until("rows") { Listing($0).selected == second }
            await app.send(["rename \(second) build server"])
            let renamed = await app.until("rows") { Listing($0).row(second)?.title == "build server" }
            app.expect(renamed, contains: "\"build server\"")
            await app.closeSelected()
            let left = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            app.expect(left.row(second) == nil, "the closed terminal is gone:\n\(left.text)")
        }
    }

    @Test("groups hold terminals and fold; terminals show as tabs and back")
    @MainActor func groupsAndTabs() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t", "key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 3 }
            let made = await app.reply("group 1")
            app.expect(!made.contains("refused"), "a group of terminal 1: \(made)")
            let grouped = Listing(await app.until("rows") { !Listing($0).groups.isEmpty })
            guard let group = grouped.groups.first?.id else { return }
            let moved = await app.reply("ingroup 2 \(group)")
            app.expect(!moved.contains("refused"), "terminal 2 into the group: \(moved)")
            await app.send(["fold \(group)"])
            let folded = Listing(await app.until("rows") { Listing($0).groups.first?.folded == true })
            // A folded group lists no rows, but its terminals still exist.
            app.expect(folded.live.count == 3 && folded.rows.map(\.id) == [3], "only terminal 3 is listed:\n\(folded.text)")
            await app.send(["fold \(group)"])
            await app.until("rows") { Listing($0).groups.first?.folded == false }
            await app.send(["tabs on"])
            await app.until("state") { $0.contains("tabs=true") }
            await app.send(["tabs off"])
            await app.until("state") { $0.contains("tabs=false") }
        }
    }
}
