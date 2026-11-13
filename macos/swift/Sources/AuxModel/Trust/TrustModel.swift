// The project trust window's data (`features/project/trust_window.rs`, core `project::trust`): every line a project
// wants to run, in full, with what runs set apart; the same approval the notice offers. Nothing runs until
// the person approves, and approving asks Touch ID (or the password) through the backend.

import Foundation

public enum TrustStatus: Sendable {
    case trusted, changed, untrusted

    public var badge: String {
        switch self {
        case .trusted: AuxText.trustTrusted
        case .changed: AuxText.trustChanged
        case .untrusted: AuxText.trustNot
        }
    }
}

/// One line a project would run: "action test (key ⌃⌘T):", `cargo test`, "(current)". Kept in three pieces so the
/// window sets what runs in its own colour (`TrustLine`).
public struct TrustLine: Equatable, Sendable, Identifiable {
    public var what: String
    public var runs: String
    public var note: String
    public var id: String { what + "\u{0}" + runs + "\u{0}" + note }

    public init(what: String, runs: String = "", note: String = "") { (self.what, self.runs, self.note) = (what, runs, note) }
}

public struct TrustReview: Equatable, Sendable {
    public var name: String
    public var directory: String
    public var status: TrustStatus
    public var lines: [TrustLine]
    /// Why there is nothing to review ("This project is no longer open in a terminal.").
    public var problem: String?
    /// The approval dialog is up.
    public var approving: Bool

    public init(name: String, directory: String, status: TrustStatus, lines: [TrustLine], problem: String? = nil, approving: Bool = false) {
        (self.name, self.directory, self.status, self.lines, self.problem, self.approving) = (name, directory, status, lines, problem, approving)
    }

    public var canApprove: Bool { status != .trusted && problem == nil }

    /// What the header says: trusted, changed since, or waiting.
    public var summary: String {
        switch status {
        case .trusted: AuxText.trustSummaryTrusted
        case .changed: AuxText.trustSummaryChanged
        case .untrusted: AuxText.trustSummaryUntrusted
        }
    }
}

@MainActor
public protocol TrustBackend: AnyObject {
    /// The project as the hub holds it now (a `git pull` while the window is open shows what would run now).
    func review() -> TrustReview
    func changes() -> AsyncStream<Void>
    /// Approves with Touch ID, or the password where there is none.
    func approve() async
    var biometricsAvailable: Bool { get }
}

/// The window for a project that is not open in any terminal now: it says so and offers nothing to approve
/// (`features/project/trust_window.rs` `Shown::of`).
@MainActor
public final class ClosedProjectTrust: TrustBackend {
    private let name: String
    public init(name: String = "") { self.name = name }
    public func review() -> TrustReview {
        TrustReview(name: name, directory: "", status: .untrusted, lines: [], problem: AuxText.trustClosed)
    }
    public func changes() -> AsyncStream<Void> { AsyncStream { _ in } }
    public func approve() async {}
    public var biometricsAvailable: Bool { false }
}
