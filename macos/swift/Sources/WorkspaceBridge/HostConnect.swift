// Starting a terminal's program in a session host, or attaching to one that runs: blocking (a host starts in about 15 ms,
// a stuck one takes seconds), so never on the main actor. The pane is blank until it is done, as in the GPUI app.

import Foundation
import Synchronization
import UniFFISource
import WorkspaceModel
import WormholeBindings

/// What a start or an attach came to.
struct Connected: Sendable {
    var terminal: Terminal?
    /// The host runs and cannot be shown: the terminal keeps its place and the bar says why.
    var unavailable: (why: HostProblem.Why, host: String)?
    /// No host would start, so the program runs in the app, which ends with it (`WorkspaceNotice.hostFallback`).
    var fallback: String?
    /// Nothing started.
    var failure: String?
    /// The terminal is the host's program of before, attached again (not a new one started in its place).
    var attached = false
}

enum HostConnect {
    /// `attach`: the host a saved terminal ran in, when it runs. A host that is gone, or none, starts a new one; one that
    /// would not start leaves the program in the app.
    static func run(
        desk: HostedTerminals, what: HostedStart, attach host: String?, visible: Bool, relay: TerminalRelay
    ) -> Connected {
        if let host {
            do {
                return keptPastQuit(Connected(terminal: try desk.attach(hostId: host, what: what, visible: visible, listener: relay), attached: true))
            } catch HostingError.Unavailable(_, let kind, _) {
                return Connected(unavailable: (HostProblem.Why(kind), host))
            } catch HostingError.Gone {
                // Its program went with its host: a new terminal where it was.
            } catch { return Connected(failure: "\(error)") }
        }
        do {
            return keptPastQuit(Connected(terminal: try desk.start(what: what, listener: relay)))
        } catch HostingError.Start(let reason), HostingError.NoProgram(let reason) {
            do { return Connected(terminal: try local(what, relay), fallback: reason) } catch { return Connected(failure: "\(error)") }
        } catch { return Connected(failure: "\(error)") }
    }

    /// Set when the app quits: a host that was being made then is let go of by the thread that made it, not by the main
    /// actor, which may be gone by then (`hosted.rs` `kept_past_quit`).
    private static let quitting = Mutex(false)
    static func quit() { quitting.withLock { $0 = true } }

    /// `ready`, let go of when the app quit while it was being made.
    private static func keptPastQuit(_ ready: Connected) -> Connected {
        if quitting.withLock({ $0 }) { _ = ready.terminal?.detach() }
        return ready
    }

    /// The same program in the app's own process.
    static func local(_ what: HostedStart, _ relay: TerminalRelay) throws -> Terminal {
        try Terminal.startPlanned(what: what, listener: relay)
    }
}

extension HostConnect {
    /// The first run's welcome: this executable as `Wormhole welcome -- <program>` with `token` in its environment alone,
    /// in the app's own process (never in a session host).
    static func welcome(_ what: HostedStart, token: String, _ relay: TerminalRelay) throws -> Terminal {
        try Terminal.startForWelcome(what: what, token: token, listener: relay)
    }
}

extension HostProblem.Why {
    init(_ core: HostUnavailableKind) {
        switch core {
        case .refused: self = .refused
        case .tooOld: self = .tooOld
        case .tooNew: self = .tooNew
        case .busy: self = .busy
        case .timedOut: self = .timedOut
        case .unreachable: self = .unreachable
        case .failed: self = .failed
        }
    }
}
