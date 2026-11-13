// What the status bar counts and how much of it fits, as the core decides it (`view::layout::status_bar`): the terminal the
// button is for (the one that has waited longest), the counts after it in the bar's order, and how much the bar says at a
// window's width. `WorkspaceBridge` installs the core's rules; without them (no FFI) the bar counts nothing and says all of
// it, so a build without the core is inert, never different.

import Foundation
import Synchronization

/// How much the bar says at a window's width.
public enum StatusBarDetail: Sendable, Comparable {
    /// Numbers, and no terminal size.
    case lean
    /// The items drop their words and keep their numbers.
    case numbers
    case words
}

/// What the bar counts: indices into the terminals it was asked about.
public struct StatusCounts: Sendable, Equatable {
    /// One count after the button: how many do `activity`, and the first of them, which a click goes to.
    public struct Item: Sendable, Equatable {
        public var activity: Activity
        public var count: Int
        public var terminal: Int

        public init(activity: Activity, count: Int, terminal: Int) { (self.activity, self.count, self.terminal) = (activity, count, terminal) }
    }

    /// Who needs input, longest waiting first: the first is the button's.
    public var needsYou: [Int]
    /// The counts after the button, in the bar's order.
    public var rest: [Item]

    public init(needsYou: [Int] = [], rest: [Item] = []) { (self.needsYou, self.rest) = (needsYou, rest) }
}

public protocol StatusRuling: Sendable {
    func counts(of terminals: [TerminalSummary]) -> StatusCounts
    func detail(windowWidth: Double) -> StatusBarDetail
}

public enum StatusRules {
    private static let installed = Mutex<(any StatusRuling)?>(nil)

    public static func install(_ rules: (any StatusRuling)?) { installed.withLock { $0 = rules } }

    public static var rules: any StatusRuling { installed.withLock { $0 } ?? Inert() }

    private struct Inert: StatusRuling {
        func counts(of terminals: [TerminalSummary]) -> StatusCounts { StatusCounts() }
        func detail(windowWidth: Double) -> StatusBarDetail { .words }
    }
}
