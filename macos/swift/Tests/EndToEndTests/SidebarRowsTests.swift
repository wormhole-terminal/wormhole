import Foundation
import Testing

/// A sidebar row's own verbs: its menu (`row-menu`, ⌃↩'s, kept unshown for `menu-pick`), and its name edited in place
/// (`rename-edit`, then keys into the field as a person types them).
extension EndToEnd {
    @Test("the row menu copies the folder with a notice, pins, groups and ungroups, splits right, shows beside and closes")
    @MainActor func rowMenu() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t"])
            guard let two = await app.live(2) else { return }
            let (first, second) = (two[0], two[1])

            let menu = await app.reply("row-menu \(first)")
            for item in ["Rename…", "Add terminal to new group", "Split right", "Close terminal", "Show beside…", "Pin to top", "Copy path"] {
                app.expect(menu, contains: item)
            }
            await app.send(["menu-pick Copy path"])
            let path = await app.pasteboard()
            app.expect(path.hasSuffix(app.scratch.root.lastPathComponent + "/home"), "Copy path copied the terminal's folder: \(path)")
            app.expect(!(await app.reply("notice chip")).contains("shown=none"), "a notice says it was copied")

            await app.send(["row-menu \(first)", "menu-pick Pin to top"])
            await app.until("rows") { Listing($0).row(first)?.pinned == true }
            await app.send(["row-menu \(first)", "menu-pick Unpin"])
            await app.until("rows") { Listing($0).row(first)?.pinned == false }

            await app.send(["row-menu \(second)", "menu-pick Add terminal to new group"])
            await app.until("rows") { Listing($0).groups.first?.count == 1 }
            app.expect(await app.reply("row-menu \(second)"), contains: "Remove from group")
            await app.send(["menu-pick Remove from group"])
            await app.until("rows") { Listing($0).groups.allSatisfy { $0.count == 0 } }

            // Show beside… lists the other terminal; choosing it makes a split of the two.
            let beside = await app.reply("row-menu \(first)")
            app.expect(beside.split(separator: "\n").contains { $0.trimmingCharacters(in: .whitespaces).hasPrefix("- ") }, "a terminal to show beside:\n\(beside)")
            await app.send(["menu-pick Show beside… > 1"])
            await app.until("panes") { Panes($0).ids.count == 2 }
            app.expect(await app.reply("row-menu \(first)"), contains: "Remove from split")
            await app.send(["menu-pick Remove from split"])
            await app.until("panes") { !Panes($0).isSplit }

            await app.send(["row-menu \(second)", "menu-pick Split right"])
            await app.until("panes") { Panes($0).ids.count == 2 }
            let three = Listing(await app.until("rows") { Listing($0).live.count == 3 })
            guard let made = three.live.first(where: { ![first, second].contains($0) }) else { return }
            await app.send(["row-menu \(made)", "menu-pick Close terminal"])
            // The core decides whether `cat` counts as running; asked or not, Close ends it.
            if (await app.question()).contains("title=") { await app.send(["question confirm"]) }
            await app.until("rows") { Listing($0).row(made) == nil }
        }
    }

    @Test("a terminal's and a group's name edited in place: Return keeps what was typed, so does the keys going elsewhere")
    @MainActor func renameInPlace() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t"])
            guard let first = await app.live(2)?.first else { return }
            let row = Listing(await app.reply("rows")).rows.firstIndex { $0.id == first }.map { $0 + 1 } ?? 1
            await app.send(["rename-edit \(row)", "wait-idle 3000", "text kept-by-return", "key return"])
            await app.until("rows") { Listing($0).row(first)?.title.contains("kept-by-return") == true }
            app.expect(await app.reply("keys"), contains: "MetalTerminalView")

            await app.send(["rename-edit \(row)", "wait-idle 3000", "text kept-by-leaving", "key f6"])
            await app.until("rows") { Listing($0).row(first)?.title.contains("kept-by-leaving") == true }

            await app.send(["group \(first)"])
            let grouped = Listing(await app.until("rows") { !Listing($0).groups.isEmpty })
            guard let group = grouped.groups.first else { return }
            await app.send(["group-menu \(group.id)", "menu-pick Rename…", "wait-idle 3000", "text Builders", "key return"])
            await app.until("rows") { Listing($0).groups.first?.name.contains("Builders") == true }
        }
    }
}
