// What the narrow title row weighs: each part's natural width, with the order the extras give way in
// (`view/layout/title_row.rs`). Nothing here measures text: the view measures and hands the widths in.

import Foundation

/// The branch chip's two widths: whole, and its glyph alone.
public struct ChipWidth: Sendable, Equatable {
    public var natural: Double
    public var icon: Double

    public init(natural: Double, icon: Double) { (self.natural, self.icon) = (natural, icon) }
}

/// What else the row can carry, each on a setting of its own. None is worth the path or the terminal's
/// name, so they give way right after the status words.
public enum Extra: Int, Sendable, CaseIterable, Equatable {
    case branchStatus, gauge, ports, pullRequest

    /// The order they go when the row is narrow, the first to go first: the pull request (about somewhere
    /// else), the ports, the agent's model and context, and last the branch's status, which belongs to the
    /// chip it follows.
    public static let giveWay: [Extra] = [.pullRequest, .ports, .gauge, .branchStatus]
}

/// How wide each extra is with the gap that separates it from what is before it; nil for one the row
/// doesn't have.
public struct Extras: Sendable, Equatable {
    private var widths: [Double?] = Array(repeating: nil, count: Extra.allCases.count)

    public init() {}

    public func with(_ extra: Extra, _ width: Double?) -> Extras {
        var copy = self
        copy.widths[extra.rawValue] = width
        return copy
    }

    public func width(_ extra: Extra) -> Double? { widths[extra.rawValue] }
    public func shows(_ extra: Extra) -> Bool { width(extra) != nil }
    var total: Double { widths.compactMap { $0 }.reduce(0, +) }
}

/// What the row wants, each part that can give way as wide as it likes to be.
public struct RowWants: Sendable, Equatable {
    /// The path's steps (`PathSteps`), widest first, each with the gap after the folder icon. Empty for a
    /// terminal without a folder.
    public var path: [Double] = []
    /// The status words ("working · 2m") with their gap, or 0 for none.
    public var words = 0.0
    public var chip: ChipWidth?
    public var extras = Extras()
    /// The ports can give way a chip at a time before they go: each is the ports' width with one chip fewer
    /// than the step before, widest first.
    public var portsNarrower: [Double] = []
    /// Open in… and the splits, and the one `⋯` button they fold into.
    public var group = 0.0
    public var folded = 0.0
    /// The terminal's name as it likes to be, and the least worth showing (about three letters).
    public var name = 0.0
    public var nameMin = 0.0
    /// Everything else, which never gives way: the folder icon, the agent's icon and status, separators, gaps.
    public var fixed = 0.0

    public init() {}
}

/// How the path is drawn.
public enum PathShown: Sendable, Equatable {
    case none
    /// The folder's icon alone.
    case icon
    /// This step of `RowWants.path`.
    case step(Int)
}

/// What the row draws.
public struct RowFit: Sendable, Equatable {
    public var path: PathShown
    public var words: Bool
    /// The chip's width; its `icon` width is the glyph alone. Nil without a branch.
    public var chip: Double?
    public var extras: Extras
    /// How many of `RowWants.portsNarrower`'s steps the ports took.
    public var portsStep: Int
    /// Open in… and the splits are one `⋯` menu.
    public var folded: Bool
    public var name: Double

    public init(path: PathShown, words: Bool, chip: Double?, extras: Extras, portsStep: Int, folded: Bool, name: Double) {
        (self.path, self.words, self.chip, self.extras) = (path, words, chip, extras)
        (self.portsStep, self.folded, self.name) = (portsStep, folded, name)
    }
}
