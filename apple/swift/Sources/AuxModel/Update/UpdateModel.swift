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

    public init(version: String, notes: String, published: Date? = nil, isSecurity: Bool = false) {
        (self.version, self.notes, self.published, self.isSecurity) = (version, notes, published, isSecurity)
    }
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
    /// The phases as they change, starting with the current one.
    func phases() -> AsyncStream<UpdatePhase>
    func check() async
    func install(_ release: Release) async
    func skip(_ release: Release)
    /// Restarts into the installed update; `whenIdle` waits for every terminal to be idle first.
    func restart(whenIdle: Bool)
    /// What a restart would cut off: the lines the restart sheet lists (empty: it restarts at once).
    func runningTerminals() -> [RunningTerminal]
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
