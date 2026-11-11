// The title row laid out as the person arranged it (`window/title/row.rs`): the parts before the flexible
// space at the row's left end, the others at its right, each only while it has something to show. Two
// crumbs side by side are joined by a chevron, buttons side by side stand flush, everything else a gap
// apart. This works out what each end holds and the room its fixed pieces take (`fixed` in `RowWants`).

import Foundation

/// The row's own measurements, each GPUI's (`window/title/*.rs`, `style/scale.rs`, `style/extras.rs`).
public enum TitleMetrics {
    /// The breadcrumb's spacing (`CRUMB_GAP`, `CRUMB_PAD`) and the gap before a button group (`ROW_GAP`).
    public static let crumbGap = 8.0
    public static let crumbPad = 8.0
    public static let rowGap = 4.0
    /// After Open in…'s narrow chevron, so its glyph stands from the split's as far as the others (`AFTER_CHEVRON`).
    public static let afterChevron = 4.0
    public static let glyph = 14.0
    public static let tile = 18.0
    public static let hit = 28.0
    /// The bar is this far in from the column's sides and the window's top, centred on the traffic lights (`BAR_INSET`).
    public static let barInset = 4.0
    /// Open in…'s chevron button (`CHEVRON_W`), the group's widths and the fold's (`OPEN_IN_W`, `SPLITS_W`, `MORE_W`).
    public static let chevronWidth = 16.0
    public static let openInWidth = hit + chevronWidth + afterChevron
    public static let splitsWidth = 2 * hit
    public static let moreWidth = hit
    /// A branch chip's glyph, gap and padding round its text, its width as the glyph alone, its height, and its
    /// widest and narrowest (`CHIP_CHROME`, `CHIP_ICON_W`, `CHIP_H`, `BRANCH_CHIP_MAX`).
    public static let chipChrome = 28.0
    public static let chipIcon = 20.0
    public static let chipHeight = 20.0
    public static let chipMax = 112.0
    /// The least of a cut text worth showing: about three letters and the ellipsis (`CRUMB_TEXT_MIN`).
    public static let textMin = 24.0
    /// The last folder is cut in its own middle when it alone is wider than this (`PATH_LAST_MAX`).
    public static let pathLastMax = 240.0
    /// A name is at most this wide (`TITLE_TAB_MAX_W`).
    public static let nameMax = 220.0
    /// Extras: a chip's height, side padding, gap and icon, the meter, the model's widest, the status dot
    /// (`style/extras.rs`).
    public static let extraPad = 8.0
    public static let extraGap = 4.0
    public static let extraIcon = 12.0
    public static let meter = 28.0
    public static let gaugeModelMax = 96.0
    public static let statusDot = 6.0
    public static let chipGap = 8.0
}

/// One end of the row: its parts in order, each with whether a chevron comes before it.
public struct TitleEnd: Sendable, Equatable {
    public var parts: [TitlePart]
    public var chevrons: [Bool]

    public init(_ parts: [TitlePart], chevrons: [Bool]) { (self.parts, self.chevrons) = (parts, chevrons) }
}

public enum TitleLayout {
    /// Whether the row draws `part` for these facts: arranged in the row and with something to show (a branch
    /// needs a repository, Open in… a folder).
    public static func gap(before: TitlePart?, _ part: TitlePart) -> Double {
        switch (before?.kind, part.kind) {
        case (nil, .button): 0
        case (nil, _): TitleMetrics.crumbPad
        case (.button, .button): before == .openIn ? TitleMetrics.afterChevron : 0
        case (.button, _), (_, .button): TitleMetrics.rowGap
        default: TitleMetrics.crumbGap
        }
    }

    /// The room after an end's last part: the breadcrumb's padding, or none after a button.
    public static func trailing(_ last: TitlePart?) -> Double {
        switch last?.kind {
        case nil, .button?: 0
        default: TitleMetrics.crumbPad
        }
    }

    /// What never gives way on the ends: the gaps before the crumbs, the chevrons, the folder's icon, the
    /// terminal's tile with its gap, and the padding. (The extras, the status words and the buttons bring
    /// their gaps with their own widths.)
    public static func fixedWidth(_ ends: (left: TitleEnd, right: TitleEnd)) -> Double {
        [ends.left, ends.right].reduce(0) { total, end in
            var width = trailing(end.parts.last)
            for (ix, part) in end.parts.enumerated() {
                if extra(part) != nil || part.kind == .button || part == .terminalStatus { continue }
                width += gap(before: ix > 0 ? end.parts[ix - 1] : nil, part)
                if end.chevrons[ix] { width += TitleMetrics.glyph + TitleMetrics.crumbGap }
                switch part {
                case .folder: width += TitleMetrics.glyph
                case .terminal: width += TitleMetrics.tile + TitleMetrics.crumbGap
                default: break
                }
            }
            return total + width
        }
    }

    /// The part's own extra that `fitRow` gives up, whose width already holds the gap before it.
    static func extra(_ part: TitlePart) -> Extra? {
        switch part {
        case .branchStatus: .branchStatus
        case .agentGauge: .gauge
        case .ports: .ports
        case .pullRequest: .pullRequest
        default: nil
        }
    }
}
