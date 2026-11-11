// The Command line tool row over core's `cli_tool`: where the `wh` script stands, Install (or Update) and Uninstall, and
// whether the folder it is in is on the PATH a new terminal gets (which asks the login shell, so off the main thread).
// Where the script goes is core's rule (`WORMHOLE_CLI_DIR`, else `/usr/local/bin` or `~/.local/bin`).

import Foundation
import SettingsModel
import WormholeBindings

extension RustSettingsHost {
    /// The running binary, which the script runs.
    private static var binary: String { Bundle.main.executablePath ?? CommandLine.arguments[0] }

    /// Reads where the tool stands, and then whether its folder is on the PATH.
    func refreshCommandLine() {
        let binary = Self.binary
        Task.detached {
            let status = cliToolStatus(binaryPath: binary)
            let record = RecordMapping.commandLine(status)
            await MainActor.run {
                self.records.commandLine = record
                self.records.commandLinePath = nil
            }
            guard status.state == .installed, let folder = status.folder else { return }
            let note = RecordMapping.pathNote(cliToolPathNote(folder: folder))
            await MainActor.run { if self.records.commandLine?.folder == folder { self.records.commandLinePath = note } }
        }
    }

    func installCommandLine() {
        let binary = Self.binary
        change { _ = try cliToolInstall(binaryPath: binary) }
    }

    func uninstallCommandLine() { change { _ = try cliToolUninstall() } }

    /// Runs a write off the main thread, says what went wrong in core's words, and reads the row again.
    private func change(_ write: @escaping @Sendable () throws -> Void) {
        Task.detached {
            let result = Result(catching: write)
            await MainActor.run {
                self.problem(result.failure.map(Self.sentence))
                self.refreshCommandLine()
            }
        }
    }

    private static func sentence(_ error: any Error) -> String {
        switch error as? CliToolError {
        case .Foreign(let message), .Failed(let message): message
        case nil: SettingsFailure.sentence(error)
        }
    }
}

extension Result {
    var failure: Failure? {
        if case .failure(let error) = self { error } else { nil }
    }
}
