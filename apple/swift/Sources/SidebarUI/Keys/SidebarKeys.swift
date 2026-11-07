// The list under the keys, as pure functions so they are tested without a view: groups and terminals are both stops,
// the rows of a folded group are skipped; ↑ ↓ move, ← folds an open group or goes from a terminal to its group, → opens a
// folded group or goes from an open one to its first terminal, ⌥↑ ⌥↓ go to the ends, letters jump to the next name
// that starts with them (core's `view::focus::list::walk`). Return opens a terminal; Space also folds a group.

import Foundation
import WorkspaceModel

enum SidebarStop: Hashable, Sendable {
    case group(GroupID)
    case row(TerminalID)
}

enum ListKey: Sendable { case up, down, left, right, first, last }

enum ListMove: Equatable, Sendable {
    case to(Int), open(Int), fold(Int), stay
}

struct SidebarWalk: Equatable, Sendable {
    struct Item: Equatable, Sendable {
        var stop: SidebarStop
        var level: Int
        /// `true` for an open group, `false` for a folded one; `nil` for a terminal.
        var isOpen: Bool?
        var name: String
    }

    var items: [Item] = []

    init(_ sections: [SidebarSection]) {
        for section in sections {
            if let head = section.head {
                items.append(Item(stop: .group(head.id), level: 0, isOpen: !head.isFolded, name: head.name))
            }
            for row in section.rows {
                items.append(Item(stop: .row(row.id), level: section.isHeadless ? 0 : 1, isOpen: nil, name: row.terminal.title))
            }
        }
    }

    func index(of stop: SidebarStop?) -> Int? { stop.flatMap { stop in items.firstIndex { $0.stop == stop } } }

    func move(from at: Int?, _ key: ListKey) -> ListMove {
        guard let last = items.indices.last else { return .stay }
        guard let at = at.map({ min($0, last) }) else {
            switch key {
            case .down, .first: return .to(0)
            case .up, .last: return .to(last)
            case .left, .right: return .stay
            }
        }
        let item = items[at]
        switch key {
        case .up: return at > 0 ? .to(at - 1) : .stay
        case .down: return at < last ? .to(at + 1) : .stay
        case .first: return .to(0)
        case .last: return .to(last)
        case .left:
            if item.isOpen == true { return .fold(at) }
            return items[..<at].lastIndex { $0.level < item.level }.map { .to($0) } ?? .stay
        case .right:
            if item.isOpen == false { return .open(at) }
            if item.isOpen == true, at < last, items[at + 1].level > item.level { return .to(at + 1) }
            return .stay
        }
    }

    /// The next name after `at` (wrapping) that starts with `typed`, ignoring case; the current one counts when it is the only match.
    func typeAhead(from at: Int?, _ typed: String) -> Int? {
        guard !typed.isEmpty, !items.isEmpty else { return nil }
        let count = items.count
        let start = at.map { $0 + 1 } ?? 0
        return (0..<count).map { (start + $0) % count }.first { items[$0].name.range(of: typed, options: [.anchored, .caseInsensitive]) != nil }
    }
}
