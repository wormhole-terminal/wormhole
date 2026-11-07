// The core's words about where a program runs and why a host failed, as the window's values and sentences.

import WorkspaceModel
import WormholeBindings

extension HostingState {
    init(_ core: TerminalHosting) {
        switch core {
        case .local: self = .local
        case .persistent(let id): self = .persistent(hostID: id)
        case .detached(let id): self = .detached(hostID: id)
        case .lost(let id): self = .lost(hostID: id)
        }
    }
}

enum HostingWords {
    /// What a terminal that cannot be shown says: the host runs, the app could not attach to it. The next launch finds the
    /// same row, so the host is not adopted again as a stranger.
    static func unavailable(_ reason: String) -> String {
        String(
            localized: "hosting-unavailable",
            defaultValue: "This terminal runs in a session host that Wormhole could not connect to: \(reason)")
    }

    /// The one-time note when terminals stay in the app because no host would start (the GPUI notice chip's detail).
    static func fallback(_ reason: String) -> String {
        String(
            localized: "hosting-fallback",
            defaultValue: "Terminals end when Wormhole quits: the session host could not start (\(reason)).")
    }

    /// The group the hosts nobody named come into (core's `RECOVERED_NAME`).
    static var recoveredGroup: String { String(localized: "hosting-recovered-group", defaultValue: "Recovered") }

    static func failure(_ error: any Error) -> String {
        if case HostingError.Unavailable(let reason, _) = error { return unavailable(reason) }
        return "\(error)"
    }
}
