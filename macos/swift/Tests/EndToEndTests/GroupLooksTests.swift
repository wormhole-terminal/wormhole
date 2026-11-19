import Foundation
import Testing

/// A group's colour set in its popover and read back from the core, a folded head's summary read from the views it built,
/// and a pinned group that keeps its place with nothing in it, over a relaunch too.
extension EndToEnd {
    /// The terminals in one group: the group's line of `rows`.
    @MainActor static func grouped(_ app: Instance, _ ids: [Int]) async -> Listing.Group? {
        await app.send(["group \(ids.map(String.init).joined(separator: " "))"])
        return Listing(await app.until("rows") { !Listing($0).groups.isEmpty }).groups.first
    }

    /// The group line of `rows` for `id`, whole (its spoken words included).
    static func groupLine(_ rows: String, _ id: Int) -> String {
        rows.split(separator: "\n").first { $0.hasPrefix("group \(id) ") }.map(String.init) ?? ""
    }

    @Test("a colour picked in the group's popover is the group's: the popover marks it, the head says it, and a relaunch keeps it")
    @MainActor func groupColourSetAndReadBack() async throws {
        try await Instance.with { app in
            guard let first = (await app.rows()).live.first, let group = await Self.grouped(app, [first]) else { return }
            await app.send(["group-popover \(group.id)"])
            await app.until("popover") { $0.contains("colours:") }
            app.expect(await app.reply("popover-colour red"), contains: "popover-colour red")
            await app.send(["popover-close"])
            // The core's words for the head name its colour (core view/sidebar/spoken.rs:121).
            let said = await app.until("rows") { Self.groupLine($0, group.id).contains("coloured red") }
            app.expect(Self.groupLine(said, group.id), contains: "coloured red")
            await app.send(["group-popover \(group.id)"])
            let again = await app.until("popover") { $0.contains("colours:") }
            app.expect(again.split(separator: "\n").first { $0.hasPrefix("colours:") }?.contains("Red [x]") == true, "the popover marks the colour:\n\(again)")
            await app.send(["popover-close", "save-windows"])
            try await app.restart()
            let back = await app.until("rows") { !Listing($0).groups.isEmpty }
            app.expect(back.split(separator: "\n").contains { $0.hasPrefix("group ") && $0.contains("coloured red") }, "the colour after a relaunch:\n\(back)")
        }
    }

    @Test("a folded group's head shows its terminals' tiles and their count; open, it shows neither")
    @MainActor func foldedSummaryShowsItsTiles() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t"])
            let live = Listing(await app.until("rows") { Listing($0).live.count == 2 }).live
            guard let group = await Self.grouped(app, live) else { return }
            await app.send(["inspect on"])
            @MainActor func onHead(_ said: String, _ id: String) -> Int {
                let all = Self.frames(said)
                guard let head = all.first(where: { $0.id == "sidebar.band" }) else { return -1 }
                return all.filter { $0.id == id && head.rect.contains(CGPoint(x: $0.rect.midX, y: $0.rect.midY)) }.count
            }
            app.expect(onHead(await app.until("marked") { onHead($0, "status.tile") == 0 }, "status.tile") == 0, "open, the head has no tiles")
            // Folded, the head keeps a tile per terminal and their count (sidebar/group_head.rs:54).
            await app.send(["fold \(group.id)"])
            let folded = await app.until("marked") { onHead($0, "status.tile") == 2 && onHead($0, "badge") == 1 }
            app.expect(onHead(folded, "status.tile") == 2 && onHead(folded, "badge") == 1, "two tiles and the count on the folded head:\n\(folded)")
            await app.send(["fold \(group.id)"])
            await app.until("marked") { onHead($0, "status.tile") == 0 && onHead($0, "badge") == 0 }
        }
    }

    @Test("a pinned group keeps its head when its last terminal closes, and after a relaunch")
    @MainActor func pinnedGroupKeepsItsPlace() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t"])
            guard let second = Listing(await app.until("rows") { Listing($0).live.count == 2 }).live.last,
                let group = await Self.grouped(app, [second])
            else { return }
            await app.send(["group-popover \(group.id)"])
            await app.until("popover") { $0.contains("Pin group") }
            await app.send(["popover-press Pin group", "popover-close"])
            await app.until("rows") { Listing($0).groups.first { $0.id == group.id }?.pinned == true }
            await app.send(["select \(second)"])
            await app.closeSelected()
            // A pinned group with nothing in it is a head with nothing under it (core view/sidebar/listing.rs:403).
            let empty = Listing(await app.until("rows") { Listing($0).live.count == 1 })
            app.expect(empty.groups.first { $0.id == group.id }.map { $0.pinned && $0.count == 0 } == true, "the empty pinned group stays:\n\(empty.text)")
            await app.send(["save-windows"])
            try await app.restart()
            let back = Listing(await app.until("rows") { !Listing($0).groups.isEmpty })
            app.expect(back.groups.contains { $0.name == group.name && $0.pinned && $0.count == 0 }, "still there after a relaunch:\n\(back.text)")
        }
    }
}
