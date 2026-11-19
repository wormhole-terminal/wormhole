import Foundation
import Testing

/// `--shell` is `WORMHOLE_SHELL` given on the command line (`main.swift`): core replaces every choice with it, a terminal's own shell
/// too, so an instance started with `--shell /bin/cat` never runs a real shell. The other shell scenarios start the instance with the
/// variable or with neither; this one is the flag itself.
extension EndToEnd {
    @Test("--shell /bin/cat forces cat on every terminal, also one opened with zsh; the shell's variable is never set")
    @MainActor func shellFlagForcesTheProgram() async throws {
        try await Instance.with(shell: .configured, config: #"{"default_shell": "/bin/bash"}"#, flags: ["--shell", "/bin/cat"]) { app in
            let shells = await app.reply("shells")
            guard let zsh = shells.split(separator: "\n").first(where: { $0.contains("zsh") })?.split(separator: "\t").first.map(String.init) else {
                return app.fail("a zsh choice:\n\(shells)")
            }
            await app.send(["new-shell \(zsh)"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.shell("echo \"zsh-version=${ZSH_VERSION:-none}\"")
            // The line is on the screen twice with `cat` (the terminal's echo, then the program's): a shell would have answered
            // with a version or `none` in place of the second.
            let line = "zsh-version=${ZSH_VERSION:-none}"
            let screen = await app.screen { $0.components(separatedBy: line).count >= 3 || $0.contains("zsh-version=5") || $0.contains("zsh-version=none") }
            app.expect(screen.components(separatedBy: line).count >= 3, "the typed line comes back as typed:\n\(screen)")
            app.expect(!screen.contains("zsh-version=5") && !screen.contains("zsh-version=none"), "no shell ran it:\n\(screen)")
        }
    }
}
