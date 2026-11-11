// The list as a drop sees it: the lines (heads and rows) with where each was measured, and the cards they stand in. From
// them the pointer's place becomes the core's own question (`DropOver`, by id: which line, which part of it, or the gap
// under a card, or the space under the list); the answer, what letting go does, is the core's (`WorkspaceStore.dropPlan`).
// After `wormhole-core` `view/sidebar/place.rs` (`Target`).

import CoreGraphics
import WorkspaceModel

struct DropLines {
    struct Line: Equatable {
        enum Kind: Equatable {
            case head(GroupID)
            case row(TerminalID)
        }
        var kind: Kind
        var frame: CGRect
        /// The card the line stands in: what a drop in the gap below it is "after".
        var card: CardRef
    }

    var lines: [Line] = []
    /// A group's card, and every row's own frame, as the views reported them.
    var cards: [GroupID: CGRect] = [:]
    var rows: [TerminalID: CGRect] = [:]
    /// The card at the bottom of the list: a drop below it is the space under the list, not a gap.
    var lastCard: CardRef?
    /// Each group's rows in order (none while folded).
    var members: [GroupID: [TerminalID]] = [:]

    init(sections: [SidebarSection], frames: [DropSlot: CGRect]) {
        for section in sections {
            if let head = section.head {
                let card = CardRef.group(head.id)
                if let frame = frames[.head(head.id)] { lines.append(Line(kind: .head(head.id), frame: frame, card: card)) }
                cards[head.id] = frames[.card(head.id)]
                members[head.id] = section.rows.map(\.id)
                lastCard = card
                append(section.rows, in: card, frames)
            } else {
                for rows in SidebarSectionView.cards(of: section.rows) {
                    let card: CardRef = rows.count > 1 ? .split(rows[0].id) : .loose(rows[0].id)
                    append(rows, in: card, frames)
                    lastCard = card
                }
            }
        }
    }

    private mutating func append(_ section: [SidebarRow], in card: CardRef, _ frames: [DropSlot: CGRect]) {
        for row in section {
            guard let frame = frames[.row(row.id)] else { continue }
            lines.append(Line(kind: .row(row.id), frame: frame, card: card))
            rows[row.id] = frame
        }
    }

    /// The line the pointer at `y` is on or nearest above, and how far down its own height it is (above the first line
    /// it is the first, at 0; below the last line's end it is past 1).
    func line(at y: CGFloat) -> (line: Line, fraction: CGFloat)? {
        guard let first = lines.first else { return nil }
        let line = lines.last { $0.frame.minY <= y } ?? first
        return (line, (y - line.frame.minY) / max(line.frame.height, 1))
    }

    /// What the pointer at `y` is over.
    func over(at y: CGFloat) -> DropOver? {
        guard let (line, fraction) = line(at: y) else { return nil }
        if y > line.frame.maxY { return line.card == lastCard ? .tail : .after(line.card) }
        let part = DropPart(fraction: fraction)
        switch line.kind {
        case .head(let group): return .head(group, part)
        case .row(let id): return .row(id, part)
        }
    }

    /// The frame of the card at the bottom of the list, for a drop under it.
    var lastFrame: CGRect? {
        switch lastCard {
        case .group(let id): cards[id]
        case .loose(let id), .split(let id): rows[id]
        case nil: nil
        }
    }
}
