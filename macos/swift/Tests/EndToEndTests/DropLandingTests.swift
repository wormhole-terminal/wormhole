import Foundation
import Testing

/// Where a dragged row or group lands. The gesture itself (the lifted row, the marks, the pointer) stays a person's check:
/// these drive the end of it through the commands that send what the list's drag sends when it is let go (`drop`, `group-drop`:
/// the same `WorkspaceAction` into the same store as `SidebarView.dragEnded`; `sidebar-let-go`: its outside branch, the window
/// asked to tear off first, else the pane area's own landing), and ask the core's plan first (`plan`, `group-plan`).
extension EndToEnd {
    /// Three loose terminals, 1 2 3 in the list.
    @MainActor static func threeTerminals(_ app: Instance) async -> [Int] {
        await app.send(["key cmd-t", "key cmd-t"])
        return Listing(await app.until("rows") { Listing($0).live.count == 3 }).live
    }

    @Test("a row let go above another moves there; let go on a group's head it joins the group")
    @MainActor func dropReordersAndGroups() async throws {
        try await Instance.with { app in
            let ids = await Self.threeTerminals(app)
            guard ids.count == 3 else { return }
            // The core plans the drop (core view/sidebar/place.rs:154): above the first row is a move.
            app.expect(await app.reply("plan \(ids[2]) row:\(ids[0]):top"), contains: "move(")
            await app.send(["drop \(ids[2]) row:\(ids[0]):top"])
            let moved = Listing(await app.until("rows") { Listing($0).rows.map(\.id) == [ids[2], ids[0], ids[1]] })
            app.expect(moved.rows.map(\.id) == [ids[2], ids[0], ids[1]], "the third row is first now:\n\(moved.text)")

            guard let group = await Self.grouped(app, [ids[0]]) else { return }
            app.expect(await app.reply("plan \(ids[1]) head:\(group.id):middle"), contains: "move(")
            await app.send(["drop \(ids[1]) head:\(group.id):middle"])
            let joined = Listing(await app.until("rows") { Listing($0).groups.first { $0.id == group.id }?.count == 2 })
            app.expect(joined.groups.first { $0.id == group.id }?.count == 2, "the group holds the dropped terminal:\n\(joined.text)")
        }
    }

    @Test("a group's head let go at the end of the list moves the group there")
    @MainActor func groupDropMovesTheGroup() async throws {
        try await Instance.with { app in
            let ids = await Self.threeTerminals(app)
            guard ids.count == 3 else { return }
            await app.send(["group \(ids[0])"])
            let first = Listing(await app.until("rows") { Listing($0).groups.count == 1 }).groups[0]
            await app.send(["group \(ids[1])"])
            let both = Listing(await app.until("rows") { Listing($0).groups.count == 2 })
            app.expect(both.groups.first?.id == first.id, "the first group is listed first:\n\(both.text)")
            // The core says where a group lands (core view/sidebar/place.rs:222).
            app.expect(!(await app.reply("group-plan \(first.id) tail")).contains("nothing"), "the end of the list is a place to go")
            await app.send(["group-drop \(first.id) tail"])
            let after = Listing(await app.until("rows") { Listing($0).groups.last?.id == first.id })
            app.expect(after.groups.last?.id == first.id, "the group moved to the end:\n\(after.text)")
        }
    }

    @Test("a row let go on another row's middle makes a split with it; let go on a pane's right edge splits that pane")
    @MainActor func splitDrops() async throws {
        try await Instance.with { app in
            let ids = await Self.threeTerminals(app)
            guard ids.count == 3 else { return }
            // On a row's middle the core's plan is a join (place.rs:154).
            app.expect(await app.reply("plan \(ids[1]) row:\(ids[0]):middle"), contains: "join(\(ids[0]))")
            await app.send(["drop \(ids[1]) row:\(ids[0]):middle", "select \(ids[0])"])
            let joined = Panes(await app.until("panes") { Panes($0).ids.count == 2 })
            app.expect(Set(joined.ids) == Set([ids[0], ids[1]]), "the two terminals share a split:\n\(joined.ids)")

            // Out of the list and over the panes: the pane under the point takes it on that edge (window/panes/drop.rs:60).
            app.expect(await app.reply("sidebar-let-go \(ids[2]) right-of \(ids[0])"), contains: "released")
            let three = Panes(await app.until("panes") { Panes($0).ids.count == 3 })
            app.expect(Set(three.ids) == Set(ids), "the let-go terminal joined the shown split:\n\(three.ids)")
        }
    }
}
