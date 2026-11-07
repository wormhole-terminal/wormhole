// The list as a drop sees it: its lines top to bottom with where each was measured, and the top level's order. Built from
// the listing and the frames the views reported, so the resolver below is plain geometry and tested without a view.
// After `wormhole-core` `view/sidebar/place.rs` (`Target`): a drop lands on a row, a group's head, the gap under a card
// or the space under the list.

import CoreGraphics
import WorkspaceModel

struct DropMap {
    struct Line {
        enum Kind: Equatable {
            case head(String)
            case row(TerminalID, group: String?)
        }
        var kind: Kind
        var frame: CGRect
    }

    var lines: [Line] = []
    /// The top level as listed: each terminal in no group and each group.
    var top: [TopItem] = []
    /// A group's visible terminals in order; a folded group has none.
    var members: [String: [TerminalID]] = [:]
    var folded: Set<String> = []
    var cards: [String: CGRect] = [:]

    init(sections: [SidebarSection], frames: [DropSlot: CGRect]) {
        for section in sections {
            if let group = section.group, !section.isHeadless {
                top.append(.group(group.id))
                folded.formUnion(section.isCollapsed ? [group.id] : [])
                members[group.id] = section.rows.map(\.id)
                if let head = frames[.head(group.id)] { lines.append(Line(kind: .head(group.id), frame: head)) }
                cards[group.id] = frames[.card(group.id)]
            }
            for row in section.rows {
                if section.isHeadless { top.append(.terminal(row.id)) }
                if let frame = frames[.row(row.id)] {
                    lines.append(Line(kind: .row(row.id, group: section.isHeadless ? nil : section.group?.id), frame: frame))
                }
            }
        }
    }

    /// The top-level item after `item`, or nil at the end.
    func next(after item: TopItem, skipping skipped: TopItem? = nil) -> TopItem? {
        guard let at = top.firstIndex(of: item) else { return nil }
        return top[(at + 1)...].first { $0 != skipped }
    }

    /// The top-level item a line belongs to.
    func item(of line: Line) -> TopItem {
        switch line.kind {
        case .head(let id): .group(id)
        case .row(let id, let group): group.map(TopItem.group) ?? .terminal(id)
        }
    }

    /// The frame of the card a top-level item stands in: a group's, or a terminal's own row.
    func span(of item: TopItem, _ line: Line) -> CGRect {
        if case .group(let id) = item, let card = cards[id] { return card }
        return line.frame
    }

    /// The line the pointer at `y` is on or nearest above, and how far down its own height it is (above the first line
    /// it is the first, at 0; below the last line's end it is past 1).
    func line(at y: CGFloat) -> (line: Line, fraction: CGFloat)? {
        guard let first = lines.first else { return nil }
        let line = lines.last { $0.frame.minY <= y } ?? first
        return (line, (y - line.frame.minY) / max(line.frame.height, 1))
    }
}
