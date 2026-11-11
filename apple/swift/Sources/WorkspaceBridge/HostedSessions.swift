// The core's persistent terminals for one window: the `HostedTerminals` object, whether it is on, and what the last run
// left to bring back. Everything it decides is Rust's (`ipc::hosts`); this only carries the answers over.

import Foundation
import WorkspaceModel
import WormholeBindings

@MainActor
final class HostedSessions {
    /// What the launch brings back, in the sidebar's order.
    enum Entry: Equatable {
        /// Its host runs: attach to it.
        case attach(hostID: String, folder: String?)
        /// Its host is gone: a new terminal in the folder it was in, drawn above what it showed (the checkpoint of the host
        /// it ran in after a restart of the Mac, else its saved text).
        case again(folder: String?, oldHost: String, text: String?)

        var folder: String? {
            switch self {
            case .attach(_, let folder), .again(let folder, _, _): folder
            }
        }
    }

    /// Whether terminals can run in session hosts: Settings' flag decides each time one starts (`enabled`).
    enum Availability {
        case on(HostedSessions)
        /// The app runs from a disk image or a temporary copy, where a host must not start: with the flag on the terminals
        /// stay in the app and the person is told to move it.
        case needsApplications(HostedTerminals)
        case off
    }

    let desk: HostedTerminals
    /// Whether this window brings in the hosts nobody names (`HostingSetup.adoptsOrphans`).
    let adoptsOrphans: Bool

    private init(desk: HostedTerminals, adoptsOrphans: Bool) { (self.desk, self.adoptsOrphans) = (desk, adoptsOrphans) }

    /// Off when there is no host program (none beside the app) or the saved terminals cannot be opened: the terminals then
    /// stay in the app.
    static func open(_ setup: HostingSetup) -> Availability {
        do {
            let desk = try HostedTerminals(dataDir: setup.dataDirectory, hostBinary: setup.hostBinary, window: setup.window)
            switch desk.program() {
            case .found: return .on(HostedSessions(desk: desk, adoptsOrphans: setup.adoptsOrphans))
            case .disposableCopy: return .needsApplications(desk)
            case .missing:
                FileHandle.standardError.write(Data("persistent terminals: no session host program beside the app\n".utf8))  // text: a diagnostic written to stderr
                return .off
            }
        } catch {
            FileHandle.standardError.write(Data("persistent terminals: \(error)\n".utf8))  // text: a diagnostic written to stderr
            return .off
        }
    }

    /// The saved terminals of the last run, in the sidebar's order, with the ids the window saved them under.
    func launchPlan() -> [(terminal: TerminalID, entry: Entry)] {
        guard let plan = try? desk.planAtLaunch() else { return [] }
        return plan.saved.map {
            (
                Int($0.terminal),
                $0.running ? .attach(hostID: $0.hostId, folder: $0.cwd) : .again(folder: $0.cwd, oldHost: $0.hostId, text: $0.text)
            )
        }
    }

    /// Whether Settings has Persistent terminals on now: asked as each terminal starts, as the GPUI app's `may_host` is.
    var enabled: Bool { desk.enabled() }

    /// The same, as entries alone.
    func launchEntries() -> [Entry] { launchPlan().map { $0.entry } }
}
