// What the bar along the bottom of a pane says (`terminal_view/pane_bar.rs`): one bar at a time, a program's
// question first, then a problem with the session host, then the last notice. The state is the terminal's (Rust's);
// this is its value, and `PaneBarActions` is what its buttons do.

import Foundation

public enum ClipboardChoice: Sendable { case once, always, deny }

/// Why a terminal's session host cannot be shown (`UnavailableKind`).
public enum HostUnavailableKind: Sendable, CaseIterable {
    case refused, tooOld, tooNew, busy, timedOut, unreachable, failed

    /// A failure that may pass by itself, so "Try again" is offered (`is_transient`).
    public var isTransient: Bool { [.busy, .timedOut, .unreachable, .failed].contains(self) }
}

public enum PaneBar: Equatable, Sendable {
    /// A program wants to read the clipboard.
    case ask(program: String)
    /// What the notice chip would have said, where the chip is off.
    case notice(BannerKind, String)
    /// The host runs and this app cannot show it; what went wrong ending it, if that failed.
    case hostUnavailable(HostUnavailableKind, endFailure: String?)
    case hostNotResponding(endFailure: String?)
}

@MainActor
public protocol PaneBarActions: AnyObject {
    func answerClipboard(_ choice: ClipboardChoice)
    func tryAttachAgain()
    /// Leave the host and start a new shell in this place.
    func openNewTerminalHere()
    func endHost(hard: Bool)
    func waitForHost()
    func restart()
    func closeTerminal()
    func dismissHostNotice()
}
