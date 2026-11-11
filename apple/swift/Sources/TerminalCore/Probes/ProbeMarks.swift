// What a Probe's rules and hooks add to a frame's rows, and the marker rows of folds: the two sections that follow the
// cell extras (`apple/FFI-CONTRACT.md`, "Probe decorations and fold markers"). Rust works them out (`probe::runtime`,
// `view::fold`); this reads them. A row the frame names and no record names has no mark.

import Foundation

/// The design tokens a Probe may paint with, in the order of `Tone::ALL`: the palette holds a slot for each.
public enum ProbeTone: Int, CaseIterable, Sendable {
    case link, danger, warning, success, muted

    /// The token the theme draws it with (`Tone::token`).
    public var token: String {
        switch self {
        case .link: "text.link"
        case .danger: "status.danger.text"
        case .warning: "status.warning.text"
        case .success: "status.success.text"
        case .muted: "text.muted"
        }
    }

    static func from(_ word: Int) -> ProbeTone? { word > 0 ? ProbeTone(rawValue: word - 1) : nil }
}

/// One rule's mark on one row: the columns `first...last`.
public struct RuleSpan: Sendable, Equatable {
    public var first: Int
    public var last: Int
    public var tone: ProbeTone?
    /// What each cell from `first` shows instead of its text (shortened or redacted); `nil` leaves the text.
    public var mask: [UInt32]?

    public init(first: Int, last: Int, tone: ProbeTone? = nil, mask: [UInt32]? = nil) {
        (self.first, self.last, self.tone, self.mask) = (first, last, tone, mask)
    }
}

/// What one row carries: a hook's tint under the whole block, and the rules' spans.
public struct RowDecor: Sendable, Equatable {
    public var block: ProbeTone?
    public var spans: [RuleSpan]
    public init(block: ProbeTone? = nil, spans: [RuleSpan] = []) { (self.block, self.spans) = (block, spans) }

    /// `extras` without the cells a mask stands in for: the real characters' marks are never shaped.
    public func withoutMasked(_ extras: [Int: CellExtra]?) -> [Int: CellExtra]? {
        guard var kept = extras else { return nil }
        for span in spans where span.mask != nil {
            for column in span.first...span.last { kept[column] = nil }
        }
        return kept
    }
}

/// A fold's marker row: the app draws it, never from output text.
public struct FoldMarker: Sendable, Equatable {
    public var id: UInt64
    public var hidden: Int
    public var isOpen: Bool
    /// The block it hides ended with a failure: the marker says so in the danger tone.
    public var failed: Bool
    public var label: String
    /// What VoiceOver says; a Probe's own fold names the Probe.
    public var spoken: String
    public var tooltip: String?

    public init(id: UInt64, hidden: Int, isOpen: Bool, failed: Bool, label: String, spoken: String, tooltip: String? = nil) {
        (self.id, self.hidden, self.isOpen, self.failed) = (id, hidden, isOpen, failed)
        (self.label, self.spoken, self.tooltip) = (label, spoken, tooltip)
    }
}

/// A marker's words while a search is open, which count the matches inside the fold.
public struct FoldWords: Sendable, Equatable {
    public var row: Int
    public var label: String
    public var spoken: String
    public init(row: Int, label: String, spoken: String) { (self.row, self.label, self.spoken) = (row, label, spoken) }
}

/// A fold's marker and the viewport row it stands on.
public struct FoldMarkerRow: Sendable, Equatable {
    public var row: Int
    public var marker: FoldMarker
    public init(row: Int, marker: FoldMarker) { (self.row, self.marker) = (row, marker) }
}

/// What a frame's sections say beyond its rows, by viewport row.
public struct FrameSections: Sendable, Equatable {
    public var extras: [Int: [Int: CellExtra]] = [:]
    public var decor: [Int: RowDecor] = [:]
    public var markers: [Int: FoldMarker] = [:]

    static let decorFlag: UInt32 = 8
    static let foldFlag: UInt32 = 16

    public static func read(_ bytes: UnsafeRawBufferPointer, header: FrameHeader) throws(FrameError) -> FrameSections {
        var words = FrameWords(bytes, header: header)
        var sections = FrameSections()
        sections.extras = try CellExtras.read(&words, header: header)
        if header.hasDecor {
            for _ in 0..<(try words.next()) {
                let (row, block, count) = (try words.next(), try words.next(), try words.next())
                var spans: [RuleSpan] = []
                for _ in 0..<count {
                    let (first, last, tone, length) = (try words.next(), try words.next(), try words.next(), try words.next())
                    var mask: [UInt32]?
                    if length > 0 {
                        var chars: [UInt32] = []
                        for _ in 0..<length { chars.append(UInt32(try words.next())) }
                        mask = chars
                    }
                    spans.append(RuleSpan(first: first, last: last, tone: .from(tone), mask: mask))
                }
                sections.decor[row] = RowDecor(block: .from(block), spans: spans)
            }
        }
        if header.hasFolds {
            for _ in 0..<(try words.next()) {
                let (row, low, high, hidden, flags) = (try words.next(), try words.next(), try words.next(), try words.next(), try words.next())
                let (label, spoken, tooltip) = (try words.text(), try words.text(), try words.text())
                sections.markers[row] = FoldMarker(
                    id: UInt64(low) | UInt64(high) << 32, hidden: hidden, isOpen: flags & 1 != 0, failed: flags & 2 != 0,
                    label: label ?? "", spoken: spoken ?? "", tooltip: tooltip)
            }
        }
        return sections
    }
}
