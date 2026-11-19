import Foundation
import Testing

/// What a terminal is started with that no other scenario shows reaching one: the login shell when nothing names a shell,
/// and a profile's git identity, agent logins and start folder (`SpawnOptions::apply_profile`: the start folder only when no
/// folder was asked for, which ⌘T always does). Read in the terminal itself: the shell says what it was given.
extension EndToEnd {
    @Test("with no shell named anywhere a terminal runs the person's login shell, as a login shell")
    @MainActor func loginShellByDefault() async throws {
        // Core's rule (`terminal::shells::login_shell`, the GPUI app's too): `$SHELL`, else `/bin/zsh`. The app is given no `SHELL`
        // here, so it is zsh whatever the account that runs the suite names (the runner's is bash: the app never asks the account).
        let path = "/bin/zsh", name = "zsh"
        try await Instance.with(shell: .configured) { app in
            // A login shell reads its profile file, which lies in the scratch home: it says so before the first prompt.
            let home = app.scratch.home
            try app.scratch.write(home, ".zprofile", "echo profile-file-was-read\n")
            try app.scratch.write(home, ".bash_profile", "echo profile-file-was-read\n")
            try app.scratch.write(home, ".profile", "echo profile-file-was-read\n")
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.screen(showing: "profile-file-was-read")
            // It is core's login shell, and it says itself that it is a login shell.
            await app.shell(#"case $0 in *zsh) [[ -o login ]] && echo LOGIN-$((1+1));; *) shopt -q login_shell && echo LOGIN-$((1+1));; esac; echo "zero=$0=""#)
            let screen = await app.screen { $0.contains("zero=/") || $0.contains("zero=-") }
            app.expect(screen, contains: "LOGIN-2")
            app.expect(screen.contains("zero=\(path)=") || screen.contains("zero=-\(name)="), "the login shell (\(path)) runs:\n\(screen)")
        }
    }

    @Test("a profile's git identity, agent logins and environment reach a terminal; its start folder the first terminal of its first window, not ⌘T's")
    @MainActor func profileIdentityLoginsAndStartFolder() async throws {
        try await Instance.with(shell: .zsh) { app in
            let start = app.scratch.home.appendingPathComponent("profile-start")
            try FileManager.default.createDirectory(at: start, withIntermediateDirectories: true)
            // The Profile page's fields, committed as it commits them.
            let said = await app.reply("profile git=Ada-Lovelace env=WH_E2E_PROFILE=from-profile start=\(start.path)")
            app.expect(said, contains: "problem=none")
            // The email and the agents' login folders, written into `profiles.json` as a person edits the file: a terminal reads
            // the list when it starts.
            let file = app.scratch.data.appendingPathComponent("profiles.json")
            app.expect(await app.waitFor(file), "the profile list is written")
            let text = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
            guard var json = try JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any], var profiles = json["profiles"] as? [[String: Any]],
                let index = profiles.firstIndex(where: { $0["id"] as? String == "default" })
            else { return app.fail("profiles.json lists the default profile:\n\(text)") }
            profiles[index]["git"] = ["name": "Ada-Lovelace", "email": "ada@example.com"]
            profiles[index]["agent_logins"] = [["agent": "claude", "dir": "~/claude-work"], ["agent": "codex", "dir": app.scratch.home.appendingPathComponent("codex-work").path]]
            json["profiles"] = profiles
            try JSONSerialization.data(withJSONObject: json, options: [.prettyPrinted, .sortedKeys]).write(to: file, options: .atomic)

            // ⌘T: the environment is the profile's; the folder is the selected terminal's, never the profile's start folder.
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            // A fact a line, each short enough for a narrow window's terminal.
            let words = #"echo "n=$GIT_AUTHOR_NAME/$GIT_COMMITTER_NAME"; echo "e=$GIT_AUTHOR_EMAIL/$GIT_COMMITTER_EMAIL"; echo "p=$WH_E2E_PROFILE="; echo "c=$CLAUDE_CONFIG_DIR"; echo "x=$CODEX_HOME"; echo "at=$PWD=""#
            await app.shell(words)
            let beside = await app.screen { $0.contains("at=/") }
            for fact in ["n=Ada-Lovelace/Ada-Lovelace", "e=ada@example.com/ada@example.com", "p=from-profile=", "c=\(app.scratch.home.path)/claude-work", "x=\(app.scratch.home.path)/codex-work"] {
                app.expect(beside.split(separator: "\n").contains { $0.trimmingCharacters(in: .whitespaces) == fact }, "\(fact) reached the terminal:\n\(beside)")
            }
            app.expect(!beside.contains("at=\(start.path)="), "⌘T opens where the selected terminal is:\n\(beside)")

            // A profile's first window has no terminal to take a folder from: its one terminal starts in that profile's start
            // folder. A second profile, written into the list as a person edits the file, entered for the first time.
            var other = profiles[index]
            other["id"] = "work"
            other["name"] = "Work"
            other["start_dir"] = start.path
            other["env"] = ["WH_E2E_PROFILE": "from-work"]
            json["profiles"] = profiles + [other]
            try JSONSerialization.data(withJSONObject: json, options: [.prettyPrinted, .sortedKeys]).write(to: file, options: .atomic)
            await app.until("profiles") { $0.contains("\"Work\"") }
            await app.send(["profile-switch work"])
            await app.until("sessions") { $0.split(separator: "\n").contains { $0.contains("profile=work ") && $0.contains("visible=true") } }
            guard let window = await app.window(inProfile: "work") else { return app.fail("a Work window in sight:\n\(await app.reply("sessions"))") }
            await app.until("rows", window: window) { Listing($0).live.count == 1 }
            // As large as the suite's windows: the new one is as large as the screen lets it be, and what it prints would wrap.
            await app.size(window: window)
            await app.send(["type " + words + "\\n"], window: window)
            let fresh = await app.until("screen", window: window) { $0.contains("at=/") }
            let logical = Instance.unwrapped(fresh).replacingOccurrences(of: "/private/tmp", with: "/tmp")
            app.expect(logical, contains: "at=\(start.path.replacingOccurrences(of: "/private/tmp", with: "/tmp"))=")
            app.expect(logical, contains: "p=from-work=")
        }
    }
}
