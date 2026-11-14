import Foundation
import Testing

/// The sidebar's groups: the folder question a new terminal in a group without a folder asks (`group-ask`, held for the control
/// folder), what its answers do to the group's members, the group keys (⌃⌥[ ] 1–9), folding, and the group's own menu.
extension EndToEnd {
    /// Two folders in the scratch home with a terminal in each, as `new <dir>` opens them: their ids, oldest first.
    @MainActor private static func terminals(in names: [String], of app: Instance) async throws -> [Int] {
        var ids: [Int] = []
        for name in names {
            let folder = app.scratch.home.appendingPathComponent(name)
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            let before = Listing(await app.reply("rows")).live
            await app.send(["new \(folder.path)"])
            let after = Listing(await app.until("rows") { Listing($0).live.count == before.count + 1 }).live
            ids += after.filter { !before.contains($0) }
        }
        return ids
    }

    @Test("a new terminal in a group without a folder asks first: Use Folder with the box sends the others away, Not Now is remembered")
    @MainActor func groupFolderQuestion() async throws {
        try await Instance.with { app in
            let made = try await Self.terminals(in: ["proj", "elsewhere"], of: app)
            guard made.count == 2 else { return app.fail("two terminals in their folders: \(made)") }
            let (inside, outside) = (made[0], made[1])
            await app.send(["group \(inside) \(outside)", "select \(inside)"])
            let grouped = Listing(await app.until("rows") { Listing($0).groups.first?.count == 2 })
            guard let group = grouped.groups.first else { return }

            // The group menu's New terminal in group goes through the question, as the + does.
            await app.send(["group-menu \(group.id)", "menu-pick New terminal in group"])
            await app.asked()
            let offer = Instance.plain(await app.reply("group-ask"))
            app.expect(offer, contains: "proj")
            app.expect(offer, contains: "leave the group")
            await app.send(["group-ask use only"])
            let answered = Listing(await app.until("rows") { Listing($0).live.count == 4 })
            app.expect(answered.groups.first?.count == 2, "the group kept the terminal in its folder and took the new one:\n\(answered.text)")
            // The one elsewhere left; the group, keeping only its folder's terminals now, is greyed in its row menu.
            let name = answered.groups.first?.name ?? group.name
            let menu = await app.reply("row-menu \(outside)")
            app.expect(menu.split(separator: "\n").contains { $0.contains("- \(name)") && $0.contains("[off]") }, "the group takes no terminal from outside:\n\(menu)")
            await app.send(["menu-close"])

            // A second group without a folder: Not Now, and the next terminal in it is not asked about.
            await app.send(["group \(outside)"])
            let two = Listing(await app.until("rows") { Listing($0).groups.count == 2 })
            guard let second = two.groups.first(where: { $0.id != group.id }) else { return }
            await app.send(["group-menu \(second.id)", "menu-pick New terminal in group"])
            await app.asked()
            await app.send(["group-ask later"])
            await app.until("rows") { Listing($0).live.count == 5 }
            await app.send(["group-menu \(second.id)", "menu-pick New terminal in group"])
            await app.until("rows") { Listing($0).live.count == 6 }
            app.expect(await app.question(), contains: "no question is up")
        }
    }

    @Test("a group deleted or renamed while its folder question is up: the terminal opens in no group and nothing is set")
    @MainActor func groupGoneBeforeTheAnswer() async throws {
        try await Instance.with { app in
            let made = try await Self.terminals(in: ["first-place", "second-place"], of: app)
            guard made.count == 2 else { return app.fail("two terminals in their folders: \(made)") }
            await app.send(["group \(made[0])", "select \(made[0])"])
            let one = Listing(await app.until("rows") { Listing($0).groups.count == 1 })
            guard let group = one.groups.first else { return }
            await app.send(["group-menu \(group.id)", "menu-pick New terminal in group"])
            await app.asked()
            await app.send(["ungroup \(group.id)", "group-ask use"])
            let loose = Listing(await app.until("rows") { Listing($0).live.count == 4 })
            app.expect(loose.groups.isEmpty, "no group came back for the answer:\n\(loose.text)")

            await app.send(["group \(made[1])", "select \(made[1])"])
            let again = Listing(await app.until("rows") { Listing($0).groups.count == 1 })
            guard let renamed = again.groups.first else { return }
            await app.send(["group-menu \(renamed.id)", "menu-pick New terminal in group"])
            await app.asked()
            await app.send(["groupname \(renamed.id) Another Name", "group-ask use"])
            let after = Listing(await app.until("rows") { Listing($0).live.count == 5 })
            app.expect(after.groups.first?.count == 1, "the renamed group did not take the terminal:\n\(after.text)")
            // Its folder was not set either: a new terminal in it still asks.
            await app.send(["group-menu \(renamed.id)", "menu-pick New terminal in group"])
            await app.asked()
            await app.send(["group-ask later"])
        }
    }

    @Test("⌃⌥] ⌃⌥[ and ⌃⌥1–9 go between groups (a folded one counts, no group is last, an empty pinned one gets a terminal); two folds settle")
    @MainActor func groupKeys() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t", "key cmd-t", "key cmd-t"])
            guard let ids = await app.live(4) else { return }
            await app.send(["group \(ids[1])"])
            await app.until("rows") { Listing($0).groups.count == 1 }
            await app.send(["group \(ids[2])"])
            let grouped = Listing(await app.until("rows") { Listing($0).groups.count == 2 })
            guard grouped.groups.count == 2 else { return }
            let (a, b) = (grouped.groups[0].id, grouped.groups[1].id)
            await app.send(["select \(ids[0])"])
            await app.until("rows") { Listing($0).selected == ids[0] }

            await app.send(["key ctrl-alt-]"])
            let next = Listing(await app.until("rows") { Listing($0).selected != ids[0] })
            app.expect(next.selected == ids[1], "from the terminals in no group (last) the next group is the first:\n\(next.text)")
            await app.send(["key ctrl-alt-]"])
            await app.until("rows") { Listing($0).selected == ids[2] }
            await app.send(["key ctrl-alt-["])
            await app.until("rows") { Listing($0).selected == ids[1] }

            // A folded group still has its number; the set in no group opens on the terminal last selected there.
            await app.send(["fold \(b)"])
            await app.until("rows") { $0.contains("group \(b) ") && Listing($0).groups.first { $0.id == b }?.folded == true }
            await app.send(["key ctrl-alt-2"])
            await app.until("rows") { Listing($0).selected == ids[2] }
            await app.send(["key ctrl-alt-3"])
            await app.until("rows") { Listing($0).selected == ids[0] }

            // Two folds at once (a double click on the chevron) end where two folds end: open, every row listed.
            await app.send(["fold \(a)", "fold \(a)"])
            let settled = Listing(await app.until("rows") { Listing($0).groups.first { $0.id == a }?.folded == false })
            app.expect(settled.row(ids[1]) != nil, "the group's row is listed again:\n\(settled.text)")

            // ⌃↩ on a group's head opens its menu; the colour dots are the right-click menu's and the popover's, not this one's.
            let menu = await app.reply("group-menu \(a)")
            app.expect(menu, contains: "Rename…")
            app.expect(menu, contains: "Close group")
            app.expect(!(await app.reply("menu-native")).contains("dots:"), "no colour dots in the keyboard's group menu")
            await app.send(["menu-close"])

            // A pinned group left empty keeps its number, and its number key starts a terminal in it.
            await app.send(["group-menu \(a)", "menu-pick Pin group"])
            await app.until("rows") { Listing($0).groups.first { $0.id == a }?.pinned == true }
            await app.send(["row-menu \(ids[1])", "menu-pick Remove from group"])
            await app.until("rows") { Listing($0).groups.first { $0.id == a }?.count == 0 }
            await app.send(["key ctrl-alt-1"])
            await app.until("rows") { Listing($0).groups.first { $0.id == a }?.count == 1 }
        }
    }
}
