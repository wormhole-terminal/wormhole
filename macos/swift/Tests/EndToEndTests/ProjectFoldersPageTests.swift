import Foundation
import Testing

/// Settings → Project folders (`SettingsUI/Projects/ProjectsPanel.swift`, GPUI `settings_window/sections/projects.rs`) driven by
/// its own switches and buttons (`settings-press` at the frames the page records), read where they act: the profile's files
/// (`project-prefs.json`, `trusted-projects.json`), the Project tab's store (`project-state`) and the project's own commands.
/// The owner's question is answered by the instance's `WORMHOLE_AUTH`, never a prompt; nothing of the person's is touched.
extension EndToEnd {
    /// The Project folders page on show, tall enough that its rows need no scrolling, once it has laid out `frame` and it stands still.
    @MainActor static func projectsPage(_ app: Instance, showing frame: String) async {
        await app.send(["settings-section projects", "settings-size 960x1500"])
        await app.settledRow(frame)
    }

    /// A project folder with a prompt (no trust needed) and an action (trust needed) in a repository of the scratch home.
    @MainActor static func pageProject(_ app: Instance, _ name: String) throws -> URL {
        let repo = try app.scratch.repository(name)
        try app.scratch.project(repo, files: ["actions.json": Self.sayAction, "prompts/review.md": "# Review\nLook at the diff.\n"])
        return repo
    }

    @Test("Project folders: a default switched off on the page is the profile's, and an open project that sets nothing follows it; on again, it is back")
    @MainActor func projectDefaultsFromThePage() async throws {
        try await Instance.with { app in
            let repo = try Self.pageProject(app, "defaults-project")
            await app.flags(["project_folders": true])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            let before = await app.until("project-state") { $0.contains("root=") && $0.contains("review") }
            app.expect(before, contains: "\"prompts=true\"")

            // "Prompts" under "Use from projects by default" (GPUI `projects.rs:273`, core `project/features.rs:207`).
            await Self.projectsPage(app, showing: "project.default.prompts")
            _ = await Self.press(app, "project.default.prompts")
            await app.waitForProfileText("project-prefs.json") { $0.range(of: #""prompts"\s*:\s*false"#, options: .regularExpression) != nil }
            let off = await app.until("project-state") { $0.contains("\"prompts=false\"") }
            app.expect(off, contains: "prompts=[]")

            _ = await Self.press(app, "project.default.prompts")
            await app.waitForProfileText("project-prefs.json") { $0.range(of: #""prompts"\s*:\s*false"#, options: .regularExpression) == nil }
            await app.until("project-state") { $0.contains("\"prompts=true\"") && $0.contains("review") }
        }
    }

    @Test("Project folders: the trusted list names a trusted project; its Revoke takes the trust away, and the project's action no longer runs, by name or by its key")
    @MainActor func trustedListRevoke() async throws {
        try await Instance.with(shell: .zsh, auth: "allow") { app in
            let repo = try Self.pageProject(app, "listed-project")
            await app.flags(["project_folders": true])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") && $0.contains("actions=[\"say\"]") }
            app.expect(await app.reply("project-run say"), contains: "project-run say")
            await app.screen(showing: "e2e-42")
            app.expect(app.profileText("trusted-projects.json").contains("listed-project"), "the approval is stored for the profile")

            // The trusted list's row and its Revoke (GPUI `projects.rs:334-345` → `features/project/trust.rs:80`).
            await Self.projectsPage(app, showing: "project.revoke.listed-project")
            _ = await Self.press(app, "project.revoke.listed-project")
            await app.until("settings-rows") { !$0.contains("project.revoke.listed-project | ") }
            await app.waitForProfileText("trusted-projects.json") { !$0.contains("listed-project") }

            // The open project hears it with nothing else done (the service tells every tab, `files_panel/project/service.rs:55`).
            let untrusted = await app.until("project-state") { $0.contains("status=untrusted") }
            app.expect(untrusted, contains: "actions=[]")
            app.expect(await app.reply("project-run say"), contains: "no such id say")
            await app.send(["key ctrl-cmd-t", "type echo mark-$((2+3))\\n"])
            await app.screen(showing: "mark-5")
            app.expect(await app.count("e2e-42") == 1, "after the revoke, neither the name nor the key ran the action")
        }
    }

    @Test("Project folders: a project's page from the trusted list; its Default | On | Off is the person's own; Revoke trust, then Trust again, from the page")
    @MainActor func projectPageChoicesAndTrust() async throws {
        try await Instance.with(auth: "allow") { app in
            let repo = try Self.pageProject(app, "paged-project")
            await app.flags(["project_folders": true])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }

            // The trusted row's Settings opens the project's own page (`projects.rs:324`).
            await Self.projectsPage(app, showing: "project.settings.paged-project")
            _ = await Self.press(app, "project.settings.paged-project")
            await app.settledRow("project.feature.prompts")
            app.expect(await app.reply("settings-rows"), contains: "project.revoke-trust | ")

            // Off for this project only (GPUI's segmented Default | On | Off, `projects.rs:501` → `trust.rs:87`).
            await app.pressAlong("project.feature.prompts", at: [(0.9, 0.5), (0.83, 0.5), (0.95, 0.5), (0.76, 0.5)]) {
                (await app.reply("project-state")).contains("\"prompts=false\"")
            }
            app.expect(await app.reply("project-state"), contains: "prompts=[]")
            await app.waitForProfileText("project-prefs.json") { $0.range(of: #""prompts"\s*:\s*false"#, options: .regularExpression) != nil }
            // Default: the choice is dropped (`clear_override`), the default (on) holds again.
            await app.pressAlong("project.feature.prompts", at: [(0.1, 0.5), (0.2, 0.5), (0.05, 0.5), (0.3, 0.5)]) {
                !app.profileText("project-prefs.json").contains("\"prompts\"")
            }
            await app.until("project-state") { $0.contains("\"prompts=true\"") && $0.contains("review") }

            // Revoke trust on the page (`projects.rs:391-398`): the tab's actions go, the page offers Trust.
            _ = await Self.press(app, "project.revoke-trust")
            let revoked = await app.until("project-state") { $0.contains("status=untrusted") }
            app.expect(revoked, contains: "actions=[]")
            app.expect(await app.reply("project-run say"), contains: "no such id say")
            await app.settledRow("project.trust")
            app.expect(!app.profileText("trusted-projects.json").contains("paged-project"), "the approval is gone from the file")

            // Trust with Touch ID, answered by the instance's own `WORMHOLE_AUTH=allow` (`projects.rs:401`).
            _ = await Self.press(app, "project.trust")
            await app.until("project-state") { $0.contains("status=trusted") && $0.contains("actions=[\"say\"]") }
            await app.settledRow("project.revoke-trust")
        }
    }
}
