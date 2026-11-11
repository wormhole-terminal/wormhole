// The updater as the UI sees it: the channel, what a check found, and where a download is
// (`update::UpdatePhase`, `UpdateChannel`, `Release`). Core downloads, verifies the minisign signature, swaps the
// bundle and restarts; this is the data and the seam (`UpdateBackend`) that the window draws and drives.

import Foundation

public enum UpdateChannel: String, CaseIterable, Sendable, Identifiable {
    case stable, preview
    public var id: String { rawValue }
}

/// A release the feed offers. `notes` is Markdown.
public struct Release: Equatable, Sendable {
    public var version: String
    public var notes: String
    /// RFC 3339 as published; only displayed.
    public var published: Date?
    public var isSecurity: Bool
    /// What the (unsigned, so only advisory) feed says the update fixes.
    public var security: Security?

    public init(version: String, notes: String, published: Date? = nil, isSecurity: Bool = false, security: Security? = nil) {
        (self.version, self.notes, self.published, self.isSecurity, self.security) = (version, notes, published, isSecurity, security)
    }
}

/// A security update's severity as the page says it: critical and high read as danger, the rest as a warning (the core words the
/// sentence), and the advisories to link.
public struct Security: Equatable, Sendable {
    public struct Advisory: Equatable, Sendable {
        public var id: String
        public var url: URL

        public init(id: String, url: URL) { (self.id, self.url) = (id, url) }
    }

    public var danger: Bool
    public var line: String
    public var advisories: [Advisory]

    public init(danger: Bool, line: String, advisories: [Advisory]) { (self.danger, self.line, self.advisories) = (danger, line, advisories) }
}

public enum UpdatePhase: Equatable, Sendable {
    case idle
    case checking
    case upToDate
    case available(Release)
    /// This build can't update itself (a development build, no feed): the reason.
    case unsupported(String)
    /// Bytes received and expected (0 while unknown).
    case downloading(received: Int64, expected: Int64)
    /// Installed and waiting for a restart.
    case installed(version: String)
    case failed(String)
}

/// What the window asks of the updater. Every call is one a button makes; the backend reports progress by
/// changing `phase` (observed through `phases`).
@MainActor
public protocol UpdateBackend: AnyObject {
    var phase: UpdatePhase { get }
    var channel: UpdateChannel { get set }
    var lastChecked: Date? { get }
    /// What a change of channel did to a build of the channel left ("The Unstable build 1.2.3 was removed ..."), when it did anything.
    var channelNote: String? { get }
    /// "Restart When Idle" was chosen and is waiting for the terminals to be idle.
    var restartPending: Bool { get }
    /// The phases as they change, starting with the current one.
    func phases() -> AsyncStream<UpdatePhase>
    func check() async
    func install(_ release: Release) async
    func skip(_ release: Release)
    /// Restarts into the installed update; `whenIdle` waits for every terminal to be idle first.
    func restart(whenIdle: Bool)
    /// Calls off a wait for the terminals to be idle ("Don't restart"); the update stays installed.
    func cancelRestartWhenIdle()
    /// What a restart would cut off: the lines the restart sheet lists (empty: it restarts at once).
    func runningTerminals() -> [RunningTerminal]
    /// `auto_update` (a check installs what it finds, and the app checks by itself) and `anonymous_count` (a check says it
    /// is the day's and the month's first): from Settings, at launch and as they change. The sample has nothing to do.
    func apply(autoUpdate: Bool, anonymousCount: Bool)
}

extension UpdateBackend {
    public func apply(autoUpdate: Bool, anonymousCount: Bool) {}
    public var channelNote: String? { nil }
    public var restartPending: Bool { false }
    public func cancelRestartWhenIdle() {}
}

/// A terminal a restart or a quit would end, as the sheet lists it.
public struct RunningTerminal: Equatable, Sendable {
    public enum Status: Sendable { case idle, busy, attention }

    public var name: String
    public var agent: String?
    public var status: Status
    public var program: String?

    public init(name: String, agent: String? = nil, status: Status, program: String? = nil) {
        (self.name, self.agent, self.status, self.program) = (name, agent, status, program)
    }
}
