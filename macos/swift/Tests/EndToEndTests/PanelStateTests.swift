import Foundation
import Testing

/// What the panel and the window hold that a person reads off them: what the stores keep per terminal (`panel-kept`), a
/// diff's extent at the tab width (`diff-header`'s `widest`), the open commit's present files (`history-state present=`), a
/// draft step (`notes draft=`), the project's problem and folder (`project-state`), the project commands' words for no project
/// (`trust-state`), the Studio's completion list (`studio-complete`), the title row's and exited bar's words, the overlay,
/// the close button's dot (`windows edited=`), the restart question, the command line's unknown command. TO-VERIFY.md lines
/// 224, 227, 287, 318, 322, 325, 422, 425, 463, 612, 613, 660, 730, 770, 788, 790, 805, 936, 958, 1137.
extension EndToEnd {
    @Test("what Files, Changes and History keep for a terminal goes when it closes; Files reads only while shown (422, 612, 613)")
    @MainActor func panelKeptPerTerminal() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("kept-repo")
            let first = await app.terminal(in: repo)
            guard let a = first.selected else { return }
            await app.send(["tab files"])
            await app.until("panel-kept") { Readout($0).first("panel-kept")?.has("watching") == true }
            await app.terminal(in: repo)
            let kept = Readout(await app.until("panel-kept") { (Readout($0).first("panel-kept")?["files"] ?? "").contains(String(a)) })
            app.expect(kept.first("panel-kept")?["roots"]?.contains("kept-repo") == true, "the folder's root is kept:\n\(kept.text)")
            await app.send(["close \(first.rows.firstIndex { $0.id == a }.map { $0 + 1 } ?? 1)"])
            await app.until("panel-kept") { !(Readout($0).first("panel-kept")?["files"] ?? "").contains(String(a)) }
            // Another tab shown: Files stops reading its folder.
            await app.send(["tab notes"])
            await app.until("panel-kept") { Readout($0).first("panel-kept")?.has("watching") == false }
        }
    }

    @Test("a diff's sideways extent follows the tab width (770)")
    @MainActor func diffExtentFollowsTabWidth() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("tabs-repo", files: ["t.txt": "a\n"])
            try app.scratch.write(repo, "t.txt", "a\n\t\t\tx\n")
            await app.terminal(in: repo)
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("t.txt") }
            await app.send(["panel-option tabwidth 2", "diff 1"])
            let narrow = Readout(await app.until("diff-header") { Readout($0).first("diff-header")?["widest"] != nil })
            let before = Int(narrow.first("diff-header")?["widest"] ?? "") ?? 0
            await app.send(["panel-option tabwidth 8"])
            await app.until("diff-header") { (Int(Readout($0).first("diff-header")?["widest"] ?? "") ?? 0) > before }
        }
    }

    @Test("an old commit's file deleted since has no open icon; the others do (463)")
    @MainActor func historyPresentFiles() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("present-repo", files: ["kept.txt": "k\n", "gone.txt": "g\n"])
            try app.scratch.git(repo, "rm", "-q", "gone.txt")
            try app.scratch.git(repo, "commit", "-q", "-m", "remove one")
            await app.terminal(in: repo)
            await app.send(["tab history"])
            await app.until("history-state") { $0.contains("commits=2") }
            await app.send(["history 2"])
            let state = await app.until("history-state") { $0.contains("present=[\"kept.txt\"]") || $0.contains("present=[\"gone.txt\", \"kept.txt\"]") }
            app.expect(state, contains: "present=[\"kept.txt\"]")
        }
    }

    @Test("a draft step goes when another terminal is selected (788)")
    @MainActor func draftStepLeaves() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t", "tab notes"])
            let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            guard let other = two.live.first(where: { $0 != two.selected }) else { return }
            await app.send(["step-draft"])
            app.expect(await app.reply("notes"), contains: "draft=true")
            await app.send(["select \(other)"])
            await app.until("notes") { $0.contains("draft=false") }
        }
    }

    @Test("a broken trusted-projects.json is said on the no-project card; an .andromeda project's head names its folder (425)")
    @MainActor func projectStoresProblem() async throws {
        try await Instance.with { app in
            await app.flags(["project_folders": true])
            await app.quit(command: "quit")
            try Data("{ broken".utf8).write(to: app.scratch.data.appendingPathComponent("trusted-projects.json"))
            try await app.start()
            let plain = app.scratch.home.appendingPathComponent("no-project")
            try FileManager.default.createDirectory(at: plain, withIntermediateDirectories: true)
            await app.terminal(in: plain)
            await app.send(["tab project"])
            let none = await app.until("project-state") { $0.contains("project=none") && $0.contains("loaded=true") }
            app.expect(!none.contains("problem=-"), "the stores' reset is said:\n\(none)")
            let old = try app.scratch.repository("old-project")
            try app.scratch.write(old, ".andromeda/config.json", "{}")
            await app.terminal(in: old)
            app.expect(await app.until("project-state") { $0.contains("root=") }, contains: "dir=.andromeda")
        }
    }

    @Test("the project commands window for no open project says so (730)")
    @MainActor func trustStateNoProject() async throws {
        try await Instance.with { app in
            // Read off the open window, so nothing while it is closed.
            app.expect(await app.reply("trust-state") == "trust-state none", "no window, no review")
            await app.send(["trust"])
            let said = Readout(await app.until("trust-state") { $0 != "trust-state none" })
            app.expect(said.first("trust-state")?["problem"] == "This project is no longer open in a terminal.", "the words:\n\(said.text)")
        }
    }

    @Test("Open terminals twice opens the layout once; the first with focus is selected; a prompt copied into a note shows Notes (224, 227, 790, 958)")
    @MainActor func projectLayoutAndPromptNote() async throws {
        try await Instance.with(auth: "allow") { app in
            await app.flags(["project_folders": true])
            let repo = try app.scratch.repository("layout-project")
            let layout = #"{"sessions": [{"name": "api", "run": "true", "focus": true}, {"name": "web", "run": "true"}]}"#
            try app.scratch.project(repo, files: ["layout.json": layout, "prompts/review.md": "# Review\nLook at it.\n"])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            let before = (await app.rows()).live.count
            await app.send(["project-layout"])
            let opened = Listing(await app.until("rows") { Listing($0).live.count == before + 2 })
            app.expect(opened.row(opened.selected ?? -1)?.title == "api", "the first with focus is selected:\n\(opened.text)")
            await app.send(["project-layout", "delay 500"])
            app.expect((await app.rows()).live.count == before + 2, "the second Open terminals opens nothing")

            await app.send(["project-note review"])
            await app.until("state") { $0.contains("tab=notes") }
            app.expect(await app.until("notes") { $0.contains("Review") }, contains: "Look at it.")
        }
    }

    @Test("Studio: mix(pri lists primary colours and the functions marked function; a finished name offers nothing (287, 1137)")
    @MainActor func studioCompletions() async throws {
        try await Instance.with { app in
            await app.send(["studio"])
            await app.until("studio-select primary") { !$0.hasPrefix("error") }
            let list = Readout(await app.reply("studio-complete mix(pri"))
            app.expect(list.all("item").contains { $0.label.hasPrefix("primary") }, "primary names:\n\(list.text)")
            let functions = Readout(await app.reply("studio-complete mi"))
            app.expect(functions.all("item").contains { $0.has("function") && $0.label.hasPrefix("mix(") }, "mix marked function:\n\(functions.text)")
            app.expect(await app.reply("studio-complete primary"), contains: "items=0")
        }
    }

    @Test("with the status bar off the title row says working with its minutes; the bar on, it says nothing (805)")
    @MainActor func titleWords() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            await app.send(["status-bar off"])
            try await app.report(agent: "codex", event: "UserPromptSubmit", terminal: 1, becomes: "working")
            app.expect(await app.until("title-words") { $0.contains("working") }, contains: "working")
            await app.send(["status-bar on"])
            await app.until("title-words") { $0 == "title-words none" }
        }
    }

    @Test("a shell picked for a terminal names itself on the exited bar (660)")
    @MainActor func exitedWordsNameTheShell() async throws {
        try await Instance.with { app in
            let shells = await app.reply("shells")
            guard let zsh = shells.split(separator: "\n").first(where: { $0.contains("zsh") })?.split(separator: "\t").first.map(String.init) else {
                return app.fail("a zsh choice:\n\(shells)")
            }
            await app.send(["new-shell \(zsh)"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.shell("exit")
            let said = await app.until("exited-words") { $0 != "exited-words none" }
            app.expect(said, contains: "zsh")
        }
    }

    @Test("the shortcut sheet opens over the window, and not under the first-run page (325)")
    @MainActor func overlayUnderOnboarding() async throws {
        try await Instance.with { app in
            _ = await Self.onboarding(app, at: "folder")
            await app.send(["key alt-cmd-/"])
            app.expect(await app.reply("overlay"), contains: "overlay none")
            await app.send(["onboarding-step close", "key alt-cmd-/"])
            await app.until("overlay") { $0 == "overlay shortcuts" }
        }
    }

    @Test("the close button's dot shows while closing would ask, and goes when nothing runs (318)")
    @MainActor func documentEditedDot() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.shell("sleep 300")
            await app.until("windows") { $0.split(separator: "\n").contains { !$0.hasPrefix("(untitled)") && $0.contains("edited=true") } }
            await app.send(["key ctrl-c"])
            await app.until("windows") { !$0.contains("edited=true") }
        }
    }

    @Test("Restart to update with a command running in a folded group asks, and names it (322)")
    @MainActor func restartAsksForFoldedRunning() async throws {
        // Only an instance with an updater has a restart to ask about (`update-installed` pretends the install).
        try await Instance.with(shell: .zsh, updater: true) { app in
            await app.shell("sleep 300")
            guard let first = (await app.rows()).live.first else { return }
            await app.send(["key cmd-t", "group \(first)"])
            guard let group = Listing(await app.until("rows") { !Listing($0).groups.isEmpty }).groups.first else { return }
            await app.send(["fold \(group.id)", "update-installed 9.9.9", "update restart"])
            app.expect(await app.asked(), contains: "sleep")
            await app.send(["question cancel"])
        }
    }

    @Test("Wormhole --json list is an unknown command with status 2, not a launch (936)")
    @MainActor func unknownCommandLine() async throws {
        try await Instance.with { app in
            let ran = try await app.cliRun(["--json", "list"], timeout: 15)
            app.expect(ran.status == 2, "status 2, got \(String(describing: ran.status)):\n\(ran.output)")
            app.expect(ran.output.lowercased().contains("unknown"), "says it is unknown:\n\(ran.output)")
            app.expect(await app.reply("sessions").split(separator: "\n").count == 1, "no second window")
        }
    }
}
