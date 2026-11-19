import CoreGraphics
import Foundation
import Testing

/// The terminal's own keys and pointer: selection, copy and paste through the instance's own clipboard file (`pasteboard`,
/// `pasteboard-set`), the context menu, Option as Meta, links under ⌘ and the text size. `/bin/cat` is the program: what is
/// typed is echoed by the line discipline on row 0 and written back by `cat` on row 1.
extension EndToEnd {
    @Test("a selection is copied by ⌘C and ⌘A, pasted by ⌘V and ⇧⌘V, the menu offers Copy and Paste, ⌘K clears")
    @MainActor func copyAndPaste() async throws {
        try await Instance.with { app in
            await app.emit("copy-me-please")
            await app.screen { $0.contains("copy-me-please") }
            let empty = await app.reply("context-menu")
            app.expect(empty.split(separator: "\n").contains { $0.hasPrefix("Copy [") && $0.contains(" off]") }, "Copy is off with nothing selected:\n\(empty)")

            await app.send(["select-cells 0 0 0 13", "key cmd-c"])
            let copied = await app.pasteboard()
            app.expect(copied.hasPrefix("copy-me-pleas"), "⌘C copied the selection: \(copied)")
            app.expect(await app.reply("notice chip"), contains: "Copied")
            let menu = await app.reply("context-menu")
            app.expect(menu.split(separator: "\n").contains { $0.hasPrefix("Copy [") && !$0.contains(" off]") }, "Copy is on with a selection:\n\(menu)")
            // The clipboard has text now, so Paste is on.
            app.expect(menu.split(separator: "\n").contains { $0.hasPrefix("Paste [") && !$0.contains(" off]") }, "Paste is on:\n\(menu)")

            // ⌘A selects the whole screen, both rows.
            await app.send(["key cmd-a", "key cmd-c"])
            let all = await app.pasteboard()
            app.expect(all.components(separatedBy: "copy-me-please").count == 3, "⌘A copied both rows: \(all)")

            await app.send(["pasteboard-set pasted-by-cmd-v", "key cmd-v"])
            await app.screen { $0.contains("pasted-by-cmd-v") }
            // ⇧⌘V is no menu's key: the terminal takes it as ⌘V (`TerminalShortcuts.shortcut(for:)`).
            await app.send(["pasteboard-set pasted-by-shift", "key shift-cmd-v"])
            await app.screen { $0.contains("pasted-by-shift") }

            // ⌥⌘C is no menu's key either: GPUI copies on it as on ⌘C (`input.rs` `platform_command`).
            await app.send(["pasteboard-set before", "select-cells 1 0 1 13", "key alt-cmd-c"])
            let option = await app.pasteboard()
            app.expect(option.hasPrefix("copy-me-pleas"), "⌥⌘C copied the selection as ⌘C does: \(option)")

            // ⌘K: Clear Buffer erases the history and sends ^L for the shell to redraw (`clearBuffer`, as the GPUI app's
            // `TerminalCommand::Clear`); `cat` is no shell, so the line discipline's echo of it is what shows it arrived.
            await app.send(["key cmd-k"])
            await app.screen { $0.contains("^L") }
        }
    }

    @Test("a ⌘ key with ⌥ or ⌃ that no row of the table uses does what the key with ⌘ alone does in the terminal")
    @MainActor func optionOrControlFallsBackToCommand() async throws {
        try await Instance.with { app in
            // ⌥⌘K and ⌃⌘K are nobody's: Clear Buffer, as ⌘K (`input.rs` `platform_command`); they used to beep.
            // Clear Buffer sends ^L to the program (and erases the history); with `cat` the echo of it is what shows.
            await app.send(["key alt-cmd-k"])
            await app.screen { $0.components(separatedBy: "^L").count - 1 == 1 }
            await app.send(["key ctrl-cmd-k"])
            await app.screen { $0.components(separatedBy: "^L").count - 1 == 2 }
            // ⌃⌘V pastes as ⌘V.
            await app.send(["pasteboard-set pasted-by-ctrl-cmd-v", "key ctrl-cmd-v"])
            await app.screen { $0.contains("pasted-by-ctrl-cmd-v") }
        }
    }

    @Test("Option as Meta follows the setting without a restart, and a synthetic Option key is Meta for left as for both")
    @MainActor func optionAsMeta() async throws {
        try await Instance.with { app in
            app.expect(Instance.value("meta", in: await app.reply("state")) == "off", "Option types characters by default")
            await app.send(["set option_as_meta=\"left\""])
            await app.until("state") { Instance.value("meta", in: $0) == "left" }
            // An injected key cannot say which Option it was: any setting but off counts it as Meta (core's `is_meta`), so the
            // program gets ESC b, which the line discipline echoes as `^[b`.
            await app.send(["key alt-b"])
            await app.screen { $0.contains("^[b") }
            await app.discardLine()
            await app.send(["set option_as_meta=\"both\""])
            await app.until("state") { Instance.value("meta", in: $0) == "both" }
            await app.send(["set option_as_meta=\"off\""])
            await app.until("state") { Instance.value("meta", in: $0) == "off" }
        }
    }

    @Test("a link: the menu offers Open Link and Copy Link, ⌘-click opens it, ⌘-click in the padding beside it does not")
    @MainActor func links() async throws {
        try await Instance.with { app in
            let address = "https://example.com/wormhole-e2e"
            await app.emit(address)
            await app.screen { $0.components(separatedBy: address).count == 3 }
            let link = await app.reply("link 1 8")
            app.expect(link, contains: "link \(address)")

            await app.send(["pointer 1 8"])
            let menu = await app.reply("context-menu")
            app.expect(menu, contains: "Open Link")
            app.expect(menu, contains: "Copy Link")

            guard let cell = await app.cellPoint(row: 0, column: 3), let frame = await app.terminalFrames().first else {
                return app.fail("no place for the terminal's cells in the accessibility tree")
            }
            // Copy Link from the menu a right-click there opens (kept unshown in a test instance).
            let right = await app.reply("right-click \(Int(cell.x)) \(Int(cell.y))")
            app.expect(right, contains: "Copy Link")
            await app.send(["menu-pick Copy Link"])
            app.expect(await app.pasteboard() == address, "Copy Link copied the address")
            app.expect(await app.reply("notice chip"), contains: "Copied")

            // ⌘-click in the 4 pt padding left of the link's first cell: nothing is opened (`launches` lists what would be).
            await app.send(["click \(Int(frame.minX) + 1) \(Int(cell.y)) cmd"])
            let none = await app.reply("launches")
            app.expect(!none.contains(address), "nothing opened from the padding:\n\(none)")
            await app.send(["click \(Int(cell.x)) \(Int(cell.y)) cmd"])
            await app.until("launches") { $0.contains(address) }
        }
    }

    @Test("⌘+ zooms every terminal of the window, a new one takes the zoom, ⌘0 resets them all")
    @MainActor func textZoom() async throws {
        try await Instance.with { app in
            @MainActor func size(of terminal: Int) async -> String {
                await app.send(["select \(terminal)"])
                return Instance.value("view", in: await app.reply("fonts")) ?? "?"
            }
            await app.send(["key cmd-t"])
            guard let two = await app.live(2) else { return }
            let (first, second) = (two[0], two[1])
            let normal = await size(of: first)
            await app.send(["key cmd-+", "key cmd-+"])
            let bigger = await size(of: second)
            app.expect(bigger != normal, "⌘+ grew the selected terminal: \(normal) → \(bigger)")
            app.expect(await size(of: first) == bigger, "the other terminal of the window took the same size")
            await app.send(["key cmd-t"])
            guard let three = await app.live(3), let made = three.first(where: { ![first, second].contains($0) }) else { return }
            app.expect(await size(of: made) == bigger, "a terminal opened into the window takes its zoom")
            await app.send(["key cmd-0"])
            for id in three { app.expect(await size(of: id) == normal, "⌘0 reset terminal \(id)") }
        }
    }
}
