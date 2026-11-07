// The core's persistent terminals for one window: the `HostedTerminals` object, whether it is on, and what the last run
// left to bring back. Everything it decides is Rust's (`ipc::hosts`); this only carries the answers over.

import Foundation
import WormholeBindings

@MainActor
final class HostedSessions {
    /// What the launch brings back, in the sidebar's order.
    enum Entry: Equatable {
        /// Its host runs: attach to it.
        case attach(hostID: String, folder: String?)
        /// Its host is gone: a new terminal in the folder it was in.
        case again(folder: String?)

        var folder: String? {
            switch self {
            case .attach(_, let folder), .again(let folder): folder
            }
        }
    }

    let desk: HostedTerminals

    /// Nil when persistent terminals are off (Settings' flag), there is no host program to start (none beside the app, or
    /// the app runs from a disk image) or the saved terminals cannot be opened: the terminals then stay in the app.
    init?(setup: HostingSetup) {
        do {
            let desk = try HostedTerminals(dataDir: setup.dataDirectory, hostBinary: setup.hostBinary, window: setup.window)
            guard desk.enabled() else { return nil }
            guard case .found = desk.program() else {
                FileHandle.standardError.write(Data("persistent terminals: no session host program: \(desk.program())\n".utf8))
                return nil
            }
            self.desk = desk
        } catch {
            FileHandle.standardError.write(Data("persistent terminals: \(error)\n".utf8))
            return nil
        }
    }

    /// The saved terminals of the last run, in the sidebar's order.
    func launchEntries() -> [Entry] {
        guard let plan = try? desk.planAtLaunch() else { return [] }
        return plan.saved.map { $0.running ? .attach(hostID: $0.hostId, folder: $0.cwd) : .again(folder: $0.cwd) }
    }
}
