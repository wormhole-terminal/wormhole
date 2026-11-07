// The terminals as the ambient surfaces list them (`notify::listing`): sections by status in the order that
// matters, the row's second line, the quick panel's search, and where ⌥⌘J goes next. A stand-in for the FFI's
// `notify` calls, which return the same values.

import Foundation

public struct AmbientSections: Sendable, Equatable {
    /// Indices into the terminals: needing you the longest waiting first, working the longest running first,
    /// finished (and failed) the newest first, and the rest as given.
    public var needsYou: [Int] = [], working: [Int] = [], finished: [Int] = [], other: [Int] = []

    public init(_ terminals: [AmbientTerminal]) {
        for (index, terminal) in terminals.enumerated() {
            switch terminal.state {
            case .needsYou: needsYou.append(index)
            case .working: working.append(index)
            case .done, .failed: finished.append(index)
            case .idle, .exited: other.append(index)
            }
        }
        // Stable: terminals that started at the same moment keep their sidebar order.
        needsYou = stable(needsYou, terminals) { $0 < $1 }
        working = stable(working, terminals) { $0 < $1 }
        finished = stable(finished, terminals) { $0 > $1 }
    }

    private func stable(_ ids: [Int], _ terminals: [AmbientTerminal], by order: (Date, Date) -> Bool) -> [Int] {
        ids.enumerated().sorted { a, b in
            let (x, y) = (terminals[a.element].since, terminals[b.element].since)
            return x == y ? a.offset < b.offset : order(x, y)
        }.map(\.element)
    }
}

extension AmbientTerminal {
    /// A terminal that nothing tells the state of and that needs input rang its bell.
    public var isBell: Bool { state == .needsYou && !source.told }

    /// The row's second line: the menu leads with the agent ("Claude Code · Approve cargo test?"), the quick
    /// panel's one-line row gives just the words.
    public func detail(withAgent: Bool) -> String? {
        let agent = withAgent ? who : nil
        let changedText = changed.flatMap { $0 > 0 ? AmbientWording.count($0, "file changed", "files changed") : nil }
        let tookText = took.map { "after \(AmbientWording.duration($0))" }
        let parts: [String?]
        switch (state, source.told) {
        case (.needsYou, true), (.working, true): parts = [agent, message]
        case (.needsYou, false): parts = ["Bell", command]
        case (.working, false): parts = [command]
        case (.done, true): parts = [agent ?? (withAgent ? nil : "Finished"), message, changedText]
        case (.done, false): parts = command == nil ? ["Finished"] : [command, tookText]
        case (.failed, true): parts = [agent, "Failed", message]
        case (.failed, false): parts = [command ?? "Failed", exit.map { "exit \($0)" }, tookText]
        case (.idle, _): parts = [who ?? folder]
        case (.exited, _): parts = ["Exited"]
        }
        let words = parts.compactMap { $0?.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        return words.isEmpty ? nil : words.joined(separator: " · ")
    }

    /// Whether this is what the quick panel's search asks for: every word is found in its name, folder, branch,
    /// command, agent or words (case doesn't matter).
    public func matches(_ query: String) -> Bool {
        let haystack = [name, folder, branch, command, who, message].compactMap { $0 }.joined(separator: " ").lowercased()
        return query.split(whereSeparator: \.isWhitespace).allSatisfy { haystack.contains($0.lowercased()) }
    }
}

extension AmbientSections {
    /// Where ⌥⌘J goes: the terminal that has waited longest; pressed again, the next one, wrapping around.
    public static func nextWaiting(_ waiting: [Int64], after last: Int64?) -> Int64? {
        guard !waiting.isEmpty else { return nil }
        let next = last.flatMap { waiting.firstIndex(of: $0) }.map { $0 + 1 } ?? 0
        return waiting[next % waiting.count]
    }
}
