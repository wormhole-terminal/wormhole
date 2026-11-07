// Splits in the list, after `wormhole-core` `view/sidebar/split.rs`: a split's members sit together,
// pinning one pins all, and each row says how it belongs (a bracket along rows that are next to each
// other, a chain mark when something sits between them).

import Foundation

public enum Bracket: Sendable, Equatable {
    case start, mid, end
}

/// What a row shows of its split.
public struct SplitMarks: Sendable, Equatable {
    public var bracket: Bracket?
    /// A member of a split that isn't together in this list: the chain mark stands in for the bracket.
    public var chain = false
    /// Another member of the selected terminal's split: it is on screen beside it.
    public var onScreen = false
    /// How many terminals the split has, this one included; 0 outside a split.
    public var members = 0

    public init(bracket: Bracket? = nil, chain: Bool = false, onScreen: Bool = false, members: Int = 0) {
        self.bracket = bracket
        self.chain = chain
        self.onScreen = onScreen
        self.members = members
    }
}

/// Everything works on positions in the window's terminal array, as core's does.
enum SplitRules {
    /// A split with only one terminal is none (a stale record, or the rest of it closed).
    static func normalised(_ ids: [Int?]) -> [Int?] {
        var sizes: [Int: Int] = [:]
        for id in ids.compactMap({ $0 }) { sizes[id, default: 0] += 1 }
        return ids.map { id in id.flatMap { sizes[$0, default: 0] > 1 ? $0 : nil } }
    }

    /// `order` with each split's members gathered at the place of its first, in the order they had.
    static func gather(_ order: [Int], _ ids: [Int?]) -> [Int] {
        var members: [Int: [Int]] = [:]
        for ix in order { if let id = ids[ix] { members[id, default: []].append(ix) } }
        var gathered: [Int] = []
        for ix in order {
            guard let id = ids[ix] else { gathered.append(ix); continue }
            // The first member brings the rest along; the others find their split gone.
            gathered += members.removeValue(forKey: id) ?? []
        }
        return gathered
    }

    /// Pinning one member pins the whole split.
    static func pinned(_ pinned: [Bool], _ ids: [Int?]) -> [Bool] {
        let pinnedSplits = Set(ids.indices.filter { pinned[$0] }.compactMap { ids[$0] })
        return ids.indices.map { pinned[$0] || ids[$0].map(pinnedSplits.contains) == true }
    }

    /// `order` with the pinned first, each block keeping its order.
    static func withPins(_ order: [Int], _ pinned: [Bool]) -> [Int] {
        order.filter { pinned[$0] } + order.filter { !pinned[$0] }
    }

    /// The marks of each visible row. `order` is every terminal as listed, `band` the group of each place
    /// in it (nil: none) and `shown` whether each terminal's row is on screen (not folded away). A split is
    /// together when its members are next to each other in `order` and in one band.
    static func marks(
        order: [Int], band: [Int?], shown: [Bool], ids: [Int?], selected: Int?
    ) -> [Int: SplitMarks] {
        var places: [Int: [Int]] = [:]
        for (at, ix) in order.enumerated() { if let id = ids[ix] { places[id, default: []].append(at) } }
        let selectedSplit = selected.flatMap { ids[$0] }
        var marks: [Int: SplitMarks] = [:]
        for (id, at) in places {
            let together = zip(at, at.dropFirst()).allSatisfy { $1 == $0 + 1 && band[$0] == band[$1] }
            let rows = at.map { order[$0] }.filter { shown[$0] }
            for (n, ix) in rows.enumerated() {
                let bracket: Bracket? =
                    together && rows.count > 1 ? (n == 0 ? .start : n == rows.count - 1 ? .end : .mid) : nil
                marks[ix] = SplitMarks(
                    bracket: bracket, chain: !together, onScreen: selectedSplit == id && selected != ix,
                    members: at.count)
            }
        }
        return marks
    }
}
