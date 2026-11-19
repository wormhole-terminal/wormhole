import Foundation
import Testing

/// A `.wormhole/` project: what it may run before and after the owner trusts it, read through `project-state` (the Project tab's
/// store) and Settings → Project folders (`project …`). The owner's question is answered by the instance's own `WORMHOLE_AUTH`.
extension EndToEnd {
    /// An action that proves it ran: the shell prints 42 only when it evaluates the line, never by echoing it.
    static let sayAction = #"[{"id": "say", "title": "Say", "type": "run", "command": "echo e2e-$((6*7))", "key": "cmd-ctrl-t"}]"#

    static let wholeProject = [
        "actions.json": sayAction,
        "config.json": #"{"font_size": 19}"#,
        "layout.json": #"{"sessions": [{"name": "dev", "run": "true"}]}"#,
        "prompts/review.md": "# Review\nLook at the diff.\n",
    ]

    @Test("an untrusted project lists what it would run and runs nothing; a cancelled approval leaves it untrusted; prompts need no trust")
    @MainActor func untrustedProject() async throws {
        try await Instance.with(auth: "cancel") { app in
            let repo = try app.scratch.repository("plain-project")
            try app.scratch.project(repo, files: Self.wholeProject)
            await app.flags(["project_folders": true])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            let state = await app.until("project-state") { $0.contains("root=") }
            app.expect(state, contains: "status=untrusted")
            app.expect(state, contains: "asks=true")
            // Held back until trusted: no action, no layout. A prompt only pastes, so it is listed.
            app.expect(state, contains: "actions=[]")
            app.expect(state, contains: "layout=[]")
            app.expect(state, contains: "review")
            let commands = Instance.value("commands", in: state).flatMap { Int($0) } ?? 0
            app.expect(commands >= 2, "what trusting would run is listed (the action, the layout's run):\n\(state)")
            app.expect(await app.reply("project-run say"), contains: "no such id say")

            // Settings → Project folders: the owner's question is cancelled, so nothing is trusted.
            guard let root = Instance.value("root", in: state) else { return }
            let known = await app.later("project list", answer: "project list")
            app.expect(known, contains: "plain-project")
            await app.later("project select \(root)", answer: "project select")
            let cancelled = await app.later("project trust", answer: "project trust")
            let trusted = cancelled.split(separator: "\n").first { $0.contains("trusted=") } ?? ""
            app.expect(!trusted.contains("plain-project"), "a cancelled approval trusts nothing:\n\(cancelled)")
            app.expect(await app.reply("project-state"), contains: "status=untrusted")

            // The person's switch for prompts takes them away; it is theirs, so it needs no trust either.
            await app.send(["project-feature prompts off"])
            let off = await app.until("project-state") { $0.contains("prompts=[]") }
            app.expect(off, contains: "prompts=false")
        }
    }

    @Test("a project trusted through the owner's answer runs its action; a change to its files asks again; revoking takes the action away")
    @MainActor func trustedProject() async throws {
        try await Instance.with(shell: .zsh, auth: "allow") { app in
            let repo = try app.scratch.repository("trusted-project")
            try app.scratch.project(repo, files: Self.wholeProject)
            await app.flags(["project_folders": true])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            let before = await app.until("project-state") { $0.contains("root=") }
            guard let root = Instance.value("root", in: before) else { return }
            await app.send(["project-approve"])
            let trusted = await app.until("project-state") { $0.contains("status=trusted") }
            app.expect(trusted, contains: "actions=[\"say\"]")
            app.expect(trusted, contains: "layout=[\"dev\"]")
            app.expect(await app.later("project list", answer: "project list"), contains: "trusted-project")

            app.expect(await app.reply("project-run say"), contains: "project-run say")
            await app.screen(showing: "e2e-42")

            // The files change after the approval: approved again before use.
            try app.scratch.project(repo, files: ["actions.json": Self.sayAction.replacingOccurrences(of: "6*7", with: "6*8")])
            await app.until("project-state") { $0.contains("status=changed") }

            // Revoked in Settings → Project folders; the Project tab, read again, lists no action and runs none.
            let revoked = await app.later("project revoke \(root)", answer: "project revoke")
            let still = revoked.split(separator: "\n").first { $0.contains("trusted=") } ?? ""
            app.expect(!still.contains("trusted-project"), "the approval is gone from Settings:\n\(revoked)")
            await app.send(["project-feature actions on"])
            let untrusted = await app.until("project-state") { $0.contains("status=untrusted") }
            app.expect(untrusted, contains: "actions=[]")
            app.expect(await app.reply("project-run say"), contains: "no such id say")
            app.expect(await app.count("e2e-42") == 1, "the action ran once, before the files changed")
        }
    }

    @Test("an action's key runs it only while the project is trusted; then the key is the terminal's again")
    @MainActor func actionKey() async throws {
        try await Instance.with(shell: .zsh, auth: "allow") { app in
            let repo = try app.scratch.repository("keyed-project")
            try app.scratch.project(repo, files: ["actions.json": Self.sayAction])
            await app.flags(["project_folders": true])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            let before = await app.until("project-state") { $0.contains("root=") }
            guard let root = Instance.value("root", in: before) else { return }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            await app.send(["key ctrl-cmd-t"])
            await app.screen(showing: "e2e-42")

            await app.later("project revoke \(root)", answer: "project revoke")
            await app.send(["project-feature actions on"])
            await app.until("project-state") { $0.contains("status=untrusted") }
            await app.send(["key ctrl-cmd-t", "type echo mark-$((2+3))\\n"])
            await app.screen(showing: "mark-5")
            app.expect(await app.count("e2e-42") == 1, "after the trust went, the key ran nothing")
        }
    }

    @Test("from a subfolder of a repository: Files and the project are the repository's, and Create makes the project at its root")
    @MainActor func projectFromASubfolder() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("init-repo", files: ["README.md": "one\n", "crates/x/lib.rs": "fn x() {}\n"])
            await app.flags(["project_folders": true, "files": true])
            await app.terminal(in: repo.appendingPathComponent("crates/x"))
            let files = await app.until("files") { $0.hasPrefix("files: /") }
            let head = files.split(separator: "\n").first.map(String.init) ?? ""
            app.expect(head.contains("/init-repo (") && !head.contains("crates"), "the tree is the repository's root:\n\(files)")

            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("project=none loaded=true") }
            await app.send(["project-init"])
            let made = repo.appendingPathComponent(".wormhole")
            app.expect(await app.waitFor(made), "the project folder is made at the repository's root: \(made.path)")
            app.expect(!FileManager.default.fileExists(atPath: repo.appendingPathComponent("crates/x/.wormhole").path), "not in the subfolder")
            let found = await app.until("project-state") { $0.contains("root=") }
            app.expect((Instance.value("root", in: found) ?? "").hasSuffix("/init-repo"), "found from the subfolder upwards:\n\(found)")

            // The folder is a project now, so its colour is the project's own: `project_color` in its config.json, not the
            // person's list in the profile's (`set_folder_choice`).
            let projectConfig = made.appendingPathComponent("config.json")
            @MainActor func projectSays(_ wanted: (String) -> Bool) async -> Bool {
                for _ in 0..<60 {
                    if wanted((try? String(contentsOf: projectConfig, encoding: .utf8)) ?? "") { return true }
                    await app.send(["delay 100"])
                }
                return false
            }
            // A line of its own: the file Create writes has the key in a comment as an example.
            let set = #"(?m)^\s*"project_color"\s*:\s*"teal""#
            app.expect(await app.reply("folder-color \(repo.path) teal"), contains: "teal")
            app.expect(await projectSays { $0.range(of: set, options: .regularExpression) != nil }, "the project's config.json holds the colour")
            app.expect(!(app.configText() ?? "").contains("init-repo"), "the profile's own list does not")
            app.expect(await app.reply("folder-color \(repo.path) none"), contains: "no colour")
            app.expect(await projectSays { $0.range(of: set, options: .regularExpression) == nil }, "taken out again")
            app.expect(await app.reply("folder-color \(repo.path) zzz"), contains: "error:")
        }
    }

    @Test("a trusted project's theme.json is the window's theme while its terminal is in front, and the person's own after")
    @MainActor func projectTheme() async throws {
        try await Instance.with(auth: "allow") { app in
            let repo = try app.scratch.repository("themed-project")
            try app.scratch.project(repo, files: ["theme.json": ##"{"name": "Themed", "extends": "Nord", "ui": {"border": "#123456", "primary": "#123456"}}"##])
            await app.flags(["project_folders": true])
            let own = await app.reply("look-keys")
            app.expect(!own.lowercased().contains("123456"), "the person's theme first:\n\(own)")
            let first = (await app.rows()).selected
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            await app.until("look-keys") { $0.lowercased().contains("#123456") }
            guard let first else { return }
            await app.send(["select \(first)"])
            await app.until("look-keys") { !$0.lowercased().contains("123456") }
        }
    }
}
