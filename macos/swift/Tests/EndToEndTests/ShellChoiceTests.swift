import Foundation
import Testing

/// Which shell a terminal runs when the instance does not force one (`Instance.with(shell: .configured)`: no
/// `WORMHOLE_SHELL`, a system shell named in `config.json` first), a trusted project's shell and environment, and a project's
/// trust following a window to another profile. TO-VERIFY.md lines 86, 87, 88, 90, 1157.
extension EndToEnd {
    @Test("default_shell /bin/bash: ⌘T runs bash as a login shell (86)")
    @MainActor func defaultShellHonoured() async throws {
        try await Instance.with(shell: .configured, config: #"{"default_shell": "/bin/bash"}"#) { app in
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.shell("shopt -q login_shell && echo LOGIN-$((1+1)); echo \"shell=$0\"")
            let screen = await app.screen(showing: "LOGIN-2")
            app.expect(screen, contains: "shell=")
            app.expect(screen.contains("bash"), "bash runs:\n\(screen)")
        }
    }

    @Test("a terminal opened with zsh runs zsh again after a relaunch, the others the default shell (87)")
    @MainActor func chosenShellKept() async throws {
        try await Instance.with(shell: .configured, config: #"{"default_shell": "/bin/bash"}"#) { app in
            let shells = await app.reply("shells")
            guard let zsh = shells.split(separator: "\n").first(where: { $0.contains("zsh") })?.split(separator: "\t").first.map(String.init) else {
                return app.fail("a zsh choice:\n\(shells)")
            }
            await app.send(["new-shell \(zsh)"])
            let listing = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            guard let chosen = listing.selected else { return }
            await app.send(["save-windows"])
            try await app.restart()
            await app.send(["select \(chosen)"])
            await app.shell("echo \"zsh-version=${ZSH_VERSION:-none}\"")
            let screen = await app.screen { $0.contains("zsh-version=5") || $0.contains("zsh-version=none") }
            app.expect(screen.contains("zsh-version=5"), "zsh again:\n\(screen)")
        }
    }

    @Test("with the shell forced to cat, a terminal of a chosen shell still runs that shell (90)")
    @MainActor func forcedShellLeavesAChoice() async throws {
        try await Instance.with { app in
            let shells = await app.reply("shells")
            guard let zsh = shells.split(separator: "\n").first(where: { $0.contains("zsh") })?.split(separator: "\t").first.map(String.init) else {
                return app.fail("a zsh choice:\n\(shells)")
            }
            await app.send(["new-shell \(zsh)"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.shell("echo \"zsh-version=${ZSH_VERSION:-none}\"")
            let screen = await app.screen { $0.contains("zsh-version=5") || $0.contains("zsh-version=none") }
            app.expect(screen.contains("zsh-version=5"), "zsh runs, not cat:\n\(screen)")
        }
    }

    @Test("a trusted project's shell and environment reach a terminal opened in it after the trust (88)")
    @MainActor func projectShellAndEnv() async throws {
        try await Instance.with(shell: .configured, auth: "allow", config: #"{"default_shell": "/bin/zsh"}"#) { app in
            await app.flags(["project_folders": true])
            let repo = try app.scratch.repository("env-project")
            try app.scratch.project(repo, files: ["config.json": #"{"env": {"E2E_X": "1"}, "shell": "/bin/bash"}"#])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            await app.terminal(in: repo)
            await app.shell("echo \"x=$E2E_X bash=${BASH_VERSION:+yes}\"")
            app.expect(await app.screen(showing: "x=1 bash=yes"), contains: "x=1 bash=yes")
        }
    }

    @Test("a window with a trusted project moved to another profile reads untrusted there, trusted again back home (1157)")
    @MainActor func projectTrustFollowsWindow() async throws {
        try await Instance.with(auth: "allow") { app in
            await app.flags(["project_folders": true])
            let repo = try app.scratch.repository("move-project")
            try app.scratch.project(repo, files: ["config.json": #"{"font_size": 15}"#])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            guard let work = await Self.newProfile(app, "Work") else { return app.fail("the Work profile") }
            await app.send(["profile-move \(work)"], window: 1)
            await app.until("project-state", window: 1) { $0.contains("status=untrusted") }
            await app.send(["profile-move default"], window: 1)
            await app.until("project-state", window: 1) { $0.contains("status=trusted") }
        }
    }
}
