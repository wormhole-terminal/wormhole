// The list under the keys: groups and terminals are both stops, the rows of a folded group are skipped. Where a key goes
// (↑ ↓ ← → ⌥↑ ⌥↓, type-ahead) is the core's (`view::focus::list`, through `Focus.rules`); this only lists what is on
// screen. Return opens a terminal; Space also folds a group.

import Foundation
import WorkspaceModel

enum SidebarStop: Hashable, Sendable {
    case group(GroupID)
    case row(TerminalID)
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
        Focus.rules.walk(items.map { ListItem(level: $0.level, isOpen: $0.isOpen) }, from: at, key)
    }

    func typeAhead(from at: Int?, _ typed: String) -> Int? { Focus.rules.typeAhead(items.map(\.name), from: at, typed) }
}
