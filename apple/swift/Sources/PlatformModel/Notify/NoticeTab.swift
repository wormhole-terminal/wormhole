// One terminal as the notification rules read it on a look (the app's `TabFacts`): what state it is in and since
// when, who tells that state, what it last said, and whether the person is looking at it. Plain values; the
// times are milliseconds on the two clocks `NoticeCoordinator` reads.

import Foundation

/// What a terminal is doing, as the ambient surfaces count it (`notify::State`).
public enum NoticeState: Sendable, Equatable {
    case idle, working, needsYou, done, failed, exited
}

/// What tells a terminal's state (`notify::Source`).
public enum NoticeSource: Sendable, Equatable {
    case plain, agent, program
}

public struct NoticeTab: Sendable, Equatable {
    public struct Failure: Sendable, Equatable {
        public var serial: UInt64
        public var code: Int32
        /// When it was told, on the monotonic clock.
        public var atMs: UInt64
        public var command: String?
        public var tookMs: UInt64?
        public var lastLine: String?
        public init(
            serial: UInt64, code: Int32, atMs: UInt64, command: String? = nil, tookMs: UInt64? = nil,
            lastLine: String? = nil
        ) {
            (self.serial, self.code, self.atMs, self.command, self.tookMs, self.lastLine) =
                (serial, code, atMs, command, tookMs, lastLine)
        }
    }

    /// A program's own notification (OSC 9, 99, 777), numbered so a new one is told without a state change.
    public struct ProgramNotice: Sendable, Equatable {
        public var serial: UInt64
        public var title: String
        public var body: String
        public init(serial: UInt64, title: String = "", body: String) { (self.serial, self.title, self.body) = (serial, title, body) }
    }

    /// The program in the foreground and when it started (Unix milliseconds).
    public struct Foreground: Sendable, Equatable {
        public var command: String
        public var startedUnixMs: UInt64
        public init(command: String, startedUnixMs: UInt64) { (self.command, self.startedUnixMs) = (command, startedUnixMs) }
    }

    /// The terminal's id as its session has it, and what its hooks report.
    public var session: Int64
    public var tab: Int64
    public var name: String
    public var state: NoticeState
    /// When it started doing that, on the monotonic clock.
    public var sinceMs: UInt64
    /// The coding agent's name, or the name of the program that reports its own status.
    public var who: String?
    public var source: NoticeSource = .plain
    public var message: String?
    /// The state is what a program reported about itself (OSC 7501), which anyone may send.
    public var reported = false
    public var failure: Failure?
    public var notice: ProgramNotice?
    public var foreground: Foreground?
    public var folder: String?
    public var branch: String?
    public var changed: UInt32?
    /// On screen in its window, and its window is the key window.
    public var selected = false
    public var key = false

    public init(session: Int64, tab: Int64, name: String, state: NoticeState, sinceMs: UInt64 = 0) {
        (self.session, self.tab, self.name, self.state, self.sinceMs) = (session, tab, name, state, sinceMs)
    }
}
