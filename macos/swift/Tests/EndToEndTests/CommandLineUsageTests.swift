import Foundation
import Testing

/// `wh --help` and `wh -h` (X08): `wh` is `exec <binary> "$@"`, so the app's executable gets the dashed word first. GPUI's
/// `main.rs:63` asks the core what a command line is (`ipc::command`), where both are the control line's usage (core
/// `ipc/control/cli/mod.rs:82`, `parse.rs:21`); the Swift app's own launch options are the dashed words the core does not know.
extension EndToEnd {
    @Test("Wormhole --help and -h print the command line's usage, as help does, not the app's launch options; no second window")
    @MainActor func dashedHelpIsTheCommandLinesUsage() async throws {
        try await Instance.with { app in
            let help = try await app.cliRun(["help"], timeout: 15)
            app.expect(help.status == 0 && help.output.contains("send-text"), "help is the usage:\n\(help.output)")
            for word in ["--help", "-h"] {
                let ran = try await app.cliRun([word], timeout: 15)
                app.expect(ran.status == 0, "\(word): status 0, got \(String(describing: ran.status))")
                app.expect(ran.output == help.output, "\(word) prints what help prints:\n\(ran.output)")
                app.expect(!ran.output.contains("--render-png"), "\(word) does not list the app's launch options:\n\(ran.output)")
            }
            // A dashed word that is neither the core's nor a launch option is refused, never a launch (GPUI: unknown command, 2).
            for word in ["--version", "--json"] {
                let ran = try await app.cliRun([word], timeout: 15)
                app.expect(ran.status == 2, "\(word): status 2, got \(String(describing: ran.status)):\n\(ran.output)")
                app.expect(
                    ran.output.contains("unknown command \"\(word)\""), "\(word) is GPUI's unknown command:\n\(ran.output)")
                app.expect(!ran.output.contains("unknown option"), "\(word) is no launch option:\n\(ran.output)")
            }
            app.expect(await app.reply("sessions").split(separator: "\n").count == 1, "no second window")
        }
    }
}
