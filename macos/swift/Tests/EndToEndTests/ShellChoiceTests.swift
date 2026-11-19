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

    // A forced shell (`WORMHOLE_SHELL`, which a test instance runs under) replaces everything, a terminal's own choice too
    // (core `terminal::process::shell`): this is what keeps typed text from ever being executed in a test instance.
    @Test("under a forced shell a terminal opened with a chosen shell still runs the forced one (90)")
    @MainActor func forcedShellLeavesAChoice() async throws {
        try await Instance.with(shell: .cat) { app in
            let shells = await app.reply("shells")
            guard let zsh = shells.split(separator: "\n").first(where: { $0.contains("zsh") })?.split(separator: "\t").first.map(String.init) else {
                return app.fail("a zsh choice:\n\(shells)")
            }
            await app.send(["new-shell \(zsh)"])
            await app.until("rows") { Listing($0).live.count == 2 }
            // cat gives the line back as typed (it is on the screen twice: the terminal's echo and cat's); a shell would have
            // answered 42.
            await app.shell("echo $((40+2))")
            let screen = await app.screen { $0.components(separatedBy: "echo $((40+2))").count >= 3 || $0.contains("\n42") }
            app.expect(!screen.contains("\n42") && !screen.hasPrefix("42"), "nothing ran the typed line:\n\(screen)")
            app.expect(screen.components(separatedBy: "echo $((40+2))").count == 3, "cat wrote the line back:\n\(screen)")
            // And the program that ends is named as cat.
            await app.send(["type \u{04}"])
            let said = await app.until("exited-words") { $0 != "exited-words none" }
            app.expect(said, contains: "cat")
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
        // zsh, so a terminal can say what environment it was started with (the project's shell is not asked for: a forced
        // shell replaces it).
        try await Instance.with(shell: .zsh, auth: "allow") { app in
            // The other profile first (making it enters it), with Project folders on in its own config.json; then back, so the
            // window that moves is one in sight (`DRIVING.md`, "Which window a command file reaches").
            guard let work = await Self.newProfile(app, "Work") else { return app.fail("the Work profile") }
            try await app.flags(["project_folders": true], profile: work)
            await app.send(["profile-switch default"])
            await app.until("sessions") { $0.contains("profile=default visible=true") }
            await app.flags(["project_folders": true])
            let repo = try app.scratch.repository("move-project")
            try app.scratch.project(repo, files: ["config.json": #"{"font_size": 15, "env": {"E2E_MOVED": "1"}}"#])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            // While trusted, a terminal opened in the project has its environment: the probe the rest relies on.
            guard let first = (await app.terminal(in: repo)).selected else { return app.fail("a terminal in the project") }
            await app.shell(#"echo "home=${E2E_MOVED:-none}""#)
            app.expect(await app.screen { $0.contains("home=1") || $0.contains("home=none") }, contains: "home=1")

            await app.send(["profile-move \(work)"])
            // The profile left opens a window of its own again; the one that moved is Work's window in sight.
            await app.until("sessions") { $0.split(separator: "\n").contains { $0.contains("profile=\(work) ") && $0.contains("visible=true") } }
            guard let moved = await app.window(inProfile: work) else { return app.fail("the moved window is in sight") }
            await app.until("project-state", window: moved) { $0.contains("status=untrusted") }
            // Wormhole is still in Default, which trusts the project: the window reads its own profile's trust.
            app.expect(await app.reply("profiles"), contains: "* default")
            // And what trust guards: a terminal opened there now gets nothing of the project's environment.
            let before = (await app.rows(window: moved)).live.count
            await app.send(["new \(repo.path)"], window: moved)
            await app.until("rows", window: moved) { Listing($0).live.count == before + 1 }
            let there = await app.rows(window: moved)
            app.expect(there.selected != nil && there.selected != first, "the new terminal is the one typed into, not the moved one:\n\(there.text)")
            await app.send([#"type echo "away=${E2E_MOVED:-none}"\n"#], window: moved)
            let away = await app.until("screen", window: moved) { $0.contains("away=1") || $0.contains("away=none") }
            app.expect(away.contains("away=none"), "no project environment in a window whose profile does not trust it:\n\(away)")

            await app.send(["profile-move default"], window: moved)
            await app.until("sessions") { !$0.split(separator: "\n").contains { $0.contains("profile=\(work) ") && $0.contains("visible=true") } }
            // Back home: the window that was untrusted, found by the terminal it holds, reads trusted again.
            let lines = (await app.windowLines()).filter { $0.contains("visible=true") }
            guard let home = lines.firstIndex(where: { line in
                (line.range(of: "terminals=[").map { line[$0.upperBound...].prefix { $0 != "]" }.split(separator: ",") } ?? []).contains { $0 == "\(first)" }
            }) else { return app.fail("the window with terminal \(first):\n\(lines.joined(separator: "\n"))") }
            app.expect(lines[home].contains("profile=default "), "it is Default's again: \(lines[home])")
            await app.until("project-state", window: home + 1) { $0.contains("status=trusted") }
        }
    }
}
