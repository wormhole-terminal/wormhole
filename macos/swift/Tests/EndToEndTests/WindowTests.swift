import Foundation
import Testing

/// The window itself: its title, the View menu's switches kept as settings across a launch, a quit by `kill`, full screen
/// coming back, and what the title row knows of the selected terminal's folder (`title`, the core's facts as the row is given
/// them; `path-menu`, the folder's menu kept unshown).
extension EndToEnd {
    @Test("the window is named after the selected terminal and follows a rename, another selection and a program's title")
    @MainActor func windowTitle() async throws {
        try await Instance.with { app in
            /// The workspace window's title (`windows`: `Title visible=… sheet=…`, the visible ones).
            @MainActor func title() async -> String {
                let lines = (await app.reply("windows")).split(separator: "\n").filter { $0.contains(" visible=true") }
                return lines.first.map { String($0[..<($0.range(of: " visible=")?.lowerBound ?? $0.endIndex)]) } ?? ""
            }
            guard let first = await app.live(1)?.first else { return }
            await app.send(["rename \(first) named-by-rename"])
            await app.until("windows") { $0.contains("named-by-rename visible=true") }
            await app.send(["key cmd-t"])
            await app.live(2)
            let other = await title()
            app.expect(!other.contains("named-by-rename"), "the window follows the new terminal: \(other)")
            await app.emit("\u{1b}]0;titled-by-program\u{07}")
            await app.until("windows") { $0.contains("titled-by-program") }
            await app.send(["select \(first)"])
            await app.until("windows") { $0.contains("named-by-rename visible=true") }
        }
    }

    @Test("View ▸ Show Status Bar and Show Terminals in Tabs are settings: config.json follows and both survive a relaunch")
    @MainActor func viewSwitchesSurvive() async throws {
        try await Instance.with { app in
            let before = Instance.value("status_bar", in: await app.reply("state"))
            let wanted = before == "true" ? "false" : "true"
            await app.send(["action toggleStatusBar"])
            await app.until("state") { Instance.value("status_bar", in: $0) == wanted }
            _ = await app.waitForConfig { $0.range(of: "\"status_bar\"\\s*:\\s*\(wanted)", options: .regularExpression) != nil }
            // Settings reads the same value (its switch follows).
            app.expect(await app.setting("status_bar"), contains: wanted)

            await app.send(["action toggleTerminalTabs"])
            await app.until("state") { Instance.value("tabs", in: $0) == "true" }
            _ = await app.waitForConfig { $0.range(of: "\"terminals_in\"\\s*:\\s*\"tabs\"", options: .regularExpression) != nil }

            try await app.restart()
            let state = await app.until("state") { Instance.value("tabs", in: $0) == "true" }
            app.expect(Instance.value("status_bar", in: state) == wanted, "the status bar's state came back:\n\(state)")
        }
    }

    @Test("`kill` of the instance quits without a question while a command runs, and the next launch restores the window")
    @MainActor func quitBySignal() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.send(["key cmd-t"])
            guard let two = await app.live(2) else { return }
            await app.shell("/bin/sleep 300")
            await app.until("quit-ask", timeout: 25) { $0.contains("sleep") }
            await app.end(by: SIGTERM)
            try await app.start()
            let back = await app.until("rows") { Listing($0).live.count == 2 }
            app.expect(Listing(back).live == two, "the same terminals came back:\n\(back)")
        }
    }

    @Test("two splits at once (⌘D twice) settle into three panes, the newest with the keys")
    @MainActor func splitTwiceAtOnce() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-d", "key cmd-d"])
            let three = Panes(await app.until("panes") { Panes($0).ids.count == 3 })
            app.expect(three.isSplit && three.focused != nil, "three panes, one with the keys:\n\(three.ids)")
            await app.send(["wait-idle 3000"])
            app.expect(Panes(await app.reply("panes")).ids.count == 3, "nothing half-open after the motion")
        }
    }

    @Test("a full-screen window is saved as one and comes back full screen")
    @MainActor func fullScreenComesBack() async throws {
        try await Instance.with { app in
            guard let line = await app.windowLines().first, let id = Instance.value("window", in: line) else { return app.fail("a window to list") }
            await app.send(["full-screen \(id)"])
            await app.until("sessions") { $0.contains("frame=fullscreen") }
            await app.send(["save-windows"])
            app.expect(await app.reply("saved-windows"), contains: "frame=fullscreen")
            try await app.restart()
            await app.until("sessions") { $0.contains("frame=fullscreen") }
        }
    }

    @Test("the title row knows a detached HEAD, a submodule's parent, a terminal hidden from agents, and an exited one's folder menu")
    @MainActor func titleRowFacts() async throws {
        try await Instance.with { app in
            let detached = try app.scratch.repository("detached-repo")
            try app.scratch.write(detached, "second.txt", "two\n")
            try app.scratch.git(detached, "add", "-A")
            try app.scratch.git(detached, "commit", "-q", "-m", "second")
            try app.scratch.git(detached, "checkout", "-q", "--detach", "HEAD~1")
            await app.send(["new \(detached.path)"])
            await app.live(2)
            await app.until("title") { $0.contains("detached: true") }

            let child = try app.scratch.repository("child-repo")
            let parent = try app.scratch.repository("parent-repo")
            try app.scratch.git(parent, "-c", "protocol.file.allow=always", "submodule", "-q", "add", child.path, "child")
            try app.scratch.git(parent, "commit", "-q", "-m", "a submodule")
            await app.send(["new \(parent.appendingPathComponent("child").path)"])
            await app.live(3)
            await app.until("title") { $0.contains("superproject: Optional(") }

            // Hide from Agents marks the title row (`agent-hide` takes the row's place in the list).
            let listed = await app.rows()
            let place = (listed.rows.firstIndex { $0.id == listed.selected } ?? 2) + 1
            await app.send(["agent-hide \(place) on"])
            await app.until("title") { $0.contains("isAgentHidden: true") }

            // A terminal whose program ended still has its folder, but nothing can be typed into it.
            let live = await app.path(menu: "Insert into terminal")
            app.expect(live?.contains("[off]") == false, "Insert into terminal is on while the program runs: \(live ?? "-")")
            await app.send(["type \u{04}"])
            await app.until("pane-bar") { $0.contains("exited") }
            let exited = await app.path(menu: "Insert into terminal")
            app.expect(exited?.contains("[off]") == true, "Insert into terminal is off for an exited terminal: \(exited ?? "-")")
        }
    }
}

extension Instance {
    /// The line of the selected terminal's folder menu (`path-menu`, kept unshown) that holds `item`, the menu then dropped.
    func path(menu item: String) async -> String? {
        let menu = await reply("path-menu")
        await send(["menu-close"])
        return menu.split(separator: "\n").first { $0.contains(item) }.map(String.init)
    }
}
