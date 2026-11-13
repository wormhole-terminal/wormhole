// What a group is painted with (core `GroupPaint`): one of the nine names, worked out from the theme, or a colour of the
// person's own (`#rrggbb`), which is theirs as chosen: it is moved along its lightness only where the theme's window or
// sidebar would hide it, and its wash only as far as the card's text needs to read on it.

import DesignSystem
import WorkspaceModel

extension GroupHead {
    /// The group's colour as its head's dot, the menus and a switcher card show it; nil for a group with none.
    public func mark(_ tokens: ThemeTokens) -> RGBA? {
        if let color { return color.mark(tokens) }
        return customColor.flatMap(RGBA.init(groupHex:)).map { GroupWash.legible($0, tokens) }
    }

    /// The colour the group's card is washed with (at `GroupWash.peak`, or faded from it).
    public func wash(_ tokens: ThemeTokens) -> RGBA? { mark(tokens).map { GroupWash.tint($0, tokens) } }

    /// The group's colour as the line under the title row (Increase Contrast): a named colour or the person's own.
    public func titleLine(_ tokens: ThemeTokens) -> RGBA? { mark(tokens).map { GroupWash.titleLine($0, tokens) } }

    /// The wash as a card outside the list fills with it: the colour at the peak's strength.
    public func cardWash(_ tokens: ThemeTokens) -> RGBA? { wash(tokens).map { $0.withAlpha(GroupWash.peak) } }
}

extension RGBA {
    /// `#rrggbb`, the form the core keeps a group's own colour in.
    init?(groupHex hex: String) {
        guard hex.hasPrefix("#"), hex.count == 7, let value = UInt32(hex.dropFirst(), radix: 16) else { return nil }
        self.init(Double(value >> 16 & 0xff) / 255, Double(value >> 8 & 0xff) / 255, Double(value & 0xff) / 255)
    }
}
