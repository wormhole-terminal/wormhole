// How many terminals are in each state, and the one state the status item shows (`notify::listing`): the most
// urgent wins, needs input before failed before finished before working before idle.

import Foundation

public enum ItemState: Sendable, Equatable {
    case idle, working, finished, failed, needsYou
}

public struct AmbientCounts: Sendable, Equatable {
    public var needsYou = 0, working = 0, finished = 0, failed = 0
    /// Idle or ended.
    public var quiet = 0

    /// The status item's count stops here ("9+"), so its width stays put.
    public static let badgeMax = 9

    public init(of terminals: [AmbientTerminal] = []) {
        for terminal in terminals {
            switch terminal.state {
            case .needsYou: needsYou += 1
            case .working: working += 1
            case .done: finished += 1
            case .failed: failed += 1
            case .idle, .exited: quiet += 1
            }
        }
    }

    /// Finished or failed and not looked at: what "needs input + finished" adds to the Dock badge.
    public var results: Int { finished + failed }
    var total: Int { needsYou + working + results + quiet }

    public var itemState: ItemState {
        if needsYou > 0 { .needsYou } else if failed > 0 { .failed } else if finished > 0 { .finished } else if working > 0 { .working } else { .idle }
    }

    /// The count beside the status item: nil for none, "9+" past nine.
    public static func badge(_ n: Int) -> String? {
        n <= 0 ? nil : n > badgeMax ? "\(badgeMax)+" : "\(n)"
    }

    /// "Wormhole — 2 need input · 1 failed · 2 working · 2 finished".
    public var tooltip: String {
        let parts = [
            needsYou > 0 ? (needsYou == 1 ? "1 needs input" : "\(needsYou) need input") : nil,
            failed > 0 ? "\(failed) failed" : nil,
            working > 0 ? "\(working) working" : nil,
            finished > 0 ? "\(finished) finished" : nil,
        ].compactMap { $0 }
        let summary =
            total == 0
            ? "no terminals open"
            : parts.isEmpty ? "\(AmbientWording.count(total, "terminal", "terminals")), all quiet" : parts.joined(separator: " · ")
        return "Wormhole — \(summary)"
    }
}
