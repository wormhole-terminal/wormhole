import Foundation

extension L10n {
    static func clipboardAsk(_ program: String) -> String { tr("terminal-clipboard-ask", "\(program) wants to read the clipboard") }
    static var programUnnamed: String { tr("terminal-program-unnamed", "A program") }
    static var clipboardOnce: String { tr("terminal-clipboard-once", "Allow once") }
    static var clipboardOnceHelp: String { tr("terminal-clipboard-once-help", "Give it the clipboard's text this time") }
    static var clipboardAlways: String { tr("terminal-clipboard-always", "Always") }
    static var clipboardAlwaysLong: String { tr("terminal-clipboard-always-long", "Always for this terminal") }
    static var clipboardAlwaysHelp: String { tr("terminal-clipboard-always-help", "Give it the clipboard whenever it asks, until this terminal closes") }
    static var clipboardDeny: String { tr("terminal-clipboard-deny", "Deny") }
    static var clipboardDenyHelp: String { tr("terminal-clipboard-deny-help", "Give it nothing") }

    static var hostsRetry: String { tr("hosts-unavailable-retry", "Try again") }
    static var hostsNew: String { tr("hosts-unavailable-new", "Open a new terminal here") }
    static var hostsEnd: String { tr("hosts-unavailable-end", "End host") }
    static var hostsNotResponding: String { tr("hosts-not-responding", "Session host not responding") }
    static var hostsWait: String { tr("hosts-not-responding-wait", "Wait") }
    static func hostsEndFailed(_ why: String) -> String { tr("hosts-end-failed", "Couldn’t end the session host: \(why)") }
    static func hostsUnavailable(_ kind: HostUnavailableKind) -> String {
        switch kind {
        case .refused: tr("hosts-unavailable-refused", "Session host unavailable: it does not accept this app")
        case .tooOld: tr("hosts-unavailable-too-old", "Session host unavailable: it is too old for this app")
        case .tooNew: tr("hosts-unavailable-too-new", "Session host unavailable: it needs a newer Wormhole")
        case .busy: tr("hosts-unavailable-busy", "Session host unavailable: it has all the connections it allows")
        case .timedOut: tr("hosts-unavailable-timed-out", "Session host unavailable: it did not answer")
        case .unreachable: tr("hosts-unavailable-unreachable", "Session host unavailable: nothing answers on its socket")
        case .failed: tr("hosts-unavailable-failed", "Session host unavailable: the connection failed")
        }
    }

    static var exitedLost: String { tr("terminal-exited-lost", "This terminal ended unexpectedly") }
    static func exited(_ shell: String) -> String { tr("terminal-exited", "\(shell) exited") }
    static func exitedWithCode(_ shell: String, _ code: Int) -> String { tr("terminal-exited-code", "\(shell) exited with code \(code)") }
    static var restart: String { tr("terminal-exited-restart", "Restart") }
    static var closeTerminal: String { tr("terminal-exited-close", "Close terminal") }
    static var dismiss: String { tr("terminal-host-notice-dismiss", "Dismiss") }
}
