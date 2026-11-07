// What the title row's extras say, apart from how wide they are (`view/title_extras.rs`): the branch's
// status, the ports a terminal listens on, and the words of each for a tooltip and for VoiceOver.

import Foundation

/// A dot while the tree has uncommitted changes, and how far the branch is from its upstream. Only the
/// departures are shown: a clean branch level with its upstream has no status.
public struct BranchStatus: Sendable, Equatable {
    public var dirty: Bool
    public var ahead: Int
    public var behind: Int
    /// `origin/main`, for the words.
    public var upstream: String?

    public init(dirty: Bool = false, ahead: Int = 0, behind: Int = 0, upstream: String? = nil) {
        (self.dirty, self.ahead, self.behind, self.upstream) = (dirty, ahead, behind, upstream)
    }

    /// "Uncommitted changes, 2 commits ahead of origin/main, 1 commit behind origin/main".
    public var words: String {
        let against = upstream.map { " \($0)" } ?? ""
        let commits = { (n: Int) in "\(n) \(n == 1 ? "commit" : "commits")" }
        return [
            dirty ? "Uncommitted changes" : nil,
            ahead > 0 ? "\(commits(ahead)) ahead of\(against)" : nil,
            behind > 0 ? "\(commits(behind)) behind\(against)" : nil,
        ].compactMap { $0 }.joined(separator: ", ")
    }
}

public enum Ports {
    /// How many ports are drawn as chips of their own; the rest are in the "+n" menu.
    public static let shown = 2

    public static func chips(_ ports: [UInt16]) -> (shown: [UInt16], more: Int) {
        (Array(ports.prefix(shown)), max(ports.count - shown, 0))
    }

    public static func label(_ port: UInt16) -> String { "localhost:\(port)" }
    /// Where a click on the chip goes: the program on this Mac, spoken to plainly.
    public static func url(_ port: UInt16) -> URL? { URL(string: "http://localhost:\(port)") }
    public static func words(_ port: UInt16) -> String { "Open localhost:\(port) in the browser" }
    public static func moreWords(_ more: [UInt16]) -> String {
        "\(more.count) more listening: \(more.map(String.init).joined(separator: ", "))"
    }
}
