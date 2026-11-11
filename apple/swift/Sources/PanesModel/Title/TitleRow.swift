// The title row's parts as the person arranged them: each at most once, the space exactly once. What comes
// before the space sits at the row's left end, the rest at its right end.

import Foundation

public struct TitleRow: Sendable, Equatable {
    public private(set) var parts: [TitlePart]

    public static let `default` = TitleRow(TitlePart.allCases)

    /// A row from any list (a hand-edited file): a part named twice keeps its first place, and a missing
    /// space goes where it is by default.
    public init(_ parts: some Sequence<TitlePart>) {
        self.parts = []
        for part in parts where !self.parts.contains(part) { self.parts.append(part) }
        show(.space)
    }

    public func shows(_ part: TitlePart) -> Bool { parts.contains(part) }
    public var isDefault: Bool { self == .default }

    /// The parts before the space (the row's left end) and after it (its right end).
    public var ends: (left: [TitlePart], right: [TitlePart]) {
        guard let at = parts.firstIndex(of: .space) else { return (parts, []) }
        return (Array(parts[..<at]), Array(parts[(at + 1)...]))
    }

    /// Show or hide `part`. A part shown again goes back where it is by default: right after the nearest
    /// part before it in the default order that is shown, else first. The space can't be hidden.
    @discardableResult
    public mutating func setShown(_ part: TitlePart, _ shown: Bool) -> Bool {
        switch (shown, shows(part)) {
        case (true, false): show(part); return true
        case (false, true) where part.hideable: parts.removeAll { $0 == part }; return true
        default: return false
        }
    }

    /// Moves the part at `from` so it ends at `to`. Whether anything moved.
    @discardableResult
    public mutating func move(from: Int, to: Int) -> Bool {
        guard from != to, parts.indices.contains(from), parts.indices.contains(to) else { return false }
        parts.insert(parts.remove(at: from), at: to)
        return true
    }

    private mutating func show(_ part: TitlePart) {
        guard !shows(part), let place = TitlePart.allCases.firstIndex(of: part) else { return }
        let current = parts
        let before = TitlePart.allCases[..<place].reversed().lazy.compactMap { current.firstIndex(of: $0) }.first
        parts.insert(part, at: before.map { $0 + 1 } ?? 0)
    }
}

/// The sample window's chevrons (the core's come with its title snapshot): whether a chevron goes before each part one end of
/// the row draws. A crumb gets one
/// when the part drawn before it, past the inline marks that belong to it, is a crumb too:
/// `folder › branch ● › terminal`, while a chip between two crumbs parts them without one.
public func chevrons(for drawn: [TitlePart]) -> [Bool] {
    var last: TitlePart.Kind?
    return drawn.map { part in
        let chevron = part.kind == .crumb && last == .crumb
        if part.kind != .inline { last = part.kind }
        return chevron
    }
}
