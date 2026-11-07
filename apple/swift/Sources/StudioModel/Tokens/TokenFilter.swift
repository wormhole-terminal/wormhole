// What the token list shows: every token, the theme's own, or the ones that fall short of a contrast minimum;
// narrowed by the gallery's area and the filter text (any part of a token's or its band's name, any case).
// A band nothing in it matches is left out (`studio/draft.rs` `visible`).

import Foundation

public struct TokenFilter: Sendable, Equatable {
    public enum Show: Sendable, Equatable, CaseIterable { case all, yours, lowContrast }

    public var show = Show.all
    public var area = StudioArea.all
    public var query = ""

    public init() {}

    /// Filtering opens every band that has a hit and stops the bands from folding.
    public var isFiltering: Bool { show != .all || area != .all || !query.trimmingCharacters(in: .whitespaces).isEmpty }

    public struct Facts {
        public let origin: (String) -> TokenOrigin
        public let low: (String) -> Bool
        public let inArea: (String, StudioArea) -> Bool

        public init(origin: @escaping (String) -> TokenOrigin, low: @escaping (String) -> Bool, inArea: @escaping (String, StudioArea) -> Bool) {
            (self.origin, self.low, self.inArea) = (origin, low, inArea)
        }
    }

    public func visible(_ groups: [TokenGroup], facts: Facts) -> [TokenGroup] {
        let needle = query.trimmingCharacters(in: .whitespaces).lowercased()
        return groups.compactMap { group in
            let groupHit = !needle.isEmpty && group.name.lowercased().contains(needle)
            let tokens = group.tokens.filter { name in
                (area == .all || facts.inArea(name, area))
                    && (show == .all || (show == .yours ? facts.origin(name) == .yours : facts.low(name)))
                    && (needle.isEmpty || groupHit || name.lowercased().contains(needle))
            }
            return tokens.isEmpty ? nil : TokenGroup(name: group.name, tokens: tokens)
        }
    }
}
