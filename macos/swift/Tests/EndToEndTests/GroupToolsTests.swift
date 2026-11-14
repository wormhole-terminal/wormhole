import Foundation
import Testing

/// The control folder's tools for a group and a row (X12, O04): the group's native menu with its row of colours, the question a
/// terminal opening in a group without a folder puts (held for `group-ask`), and a row's name edited in its field.
extension EndToEnd {
    @Test("a group's menu offers its colours as dots; a terminal opening in a folderless group asks first; a row's name is edited in place")
    @MainActor func groupTools() async throws {
        try await Instance.with { app in
            guard let first = (await app.rows()).live.first else { return }
            await app.send(["group \(first)"])
            let grouped = Listing(await app.until("rows") { !Listing($0).groups.isEmpty })
            guard let group = grouped.groups.first?.id else { return }

            app.expect(await app.reply("group-menu \(group)"), contains: "group-menu \(group)")
            let native = await app.reply("menu-native")
            app.expect(native, contains: "dots: 1. ")
            app.expect(await app.reply("menu-native press 3"), contains: "menu-native press 3: ")
            app.expect(await app.reply("menu-native press 3"), contains: "error")

            // The group has no folder of its own, and its terminal is in one: a new terminal in it asks whether to use it.
            await app.send(["newin \(group)"])
            let asked = await app.until("group-ask") { $0.contains("title=") }
            app.expect(asked, contains: "buttons=")
            app.expect(await app.reply("group-ask later"), contains: "question answered")
            await app.until("rows") { Listing($0).live.count == 2 }

            await app.send(["rename-edit 1", "text Renamed-row", "key return"])
            // The field may keep the old name before what is typed; the row's name ends with it.
            await app.until("rows") { $0.contains("Renamed-row\"") }
            app.expect(await app.reply("rename-edit 9"), contains: "error: no row 9")
        }
    }
}
