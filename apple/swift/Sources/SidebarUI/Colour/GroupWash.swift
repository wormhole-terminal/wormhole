// How a folder's or a group's colour looks in a theme, ported from `wormhole-core` `theme/folder_color.rs` and
// `view/sidebar/group_color.rs`: the names stay what they say in every theme (a hue is fixed), and only the lightness
// and chroma come from the theme's own palette, moved far enough from the window and the sidebar to be seen on both.
// `wash` is the same colour moved just far enough in lightness for a card's text to read on `WASH_PEAK` of it.

import DesignSystem
import Foundation
import WorkspaceModel

enum GroupWash {
    /// How a group's colour washes its card, at its strongest (`WASH_PEAK`), and the fade of "Fade group colours":
    /// from the peak at the card's left edge down to `far` at `span` of its width (`WASH_FAR`, `WASH_SPAN`).
    static let peak = 0.22
    static let far = 0.03
    static let span = 0.6

    /// The least a colour is kept from the surfaces it is drawn on: the 3:1 WCAG asks of a mark that isn't text.
    static let markContrast = 3.0
    /// The least chroma a colour has in a theme whose palette is nearly grey: below it "Teal" and "Green" could not
    /// be told apart.
    static let chromaMin = 0.08

    /// The text a card's wash must leave readable, and the contrast each needs: the primary text, the second lines,
    /// and the amber of a terminal that needs input (its text and its edge).
    static let text: [(token: String, ratio: Double)] = [
        ("text.primary", 4.5), ("text.muted", 3.5), ("agent.attention.text", 4.0), ("agent.attention", 3.0),
    ]

    /// The card a wash lies on: the card's own fill over the sidebar column over the window.
    static func backdrop(_ tokens: ThemeTokens) -> RGBA? {
        guard let window = tokens.rgba(named: "surface.window"), let sidebar = tokens.rgba(named: "sidebar.background"),
            let card = tokens.rgba(named: "card")
        else { return nil }
        return card.over(sidebar.over(window.over(RGBA(0, 0, 0))))
    }

    /// `mark` as its wash is made of: itself where the card's text reads on `peak` of it, else moved along its
    /// lightness toward the card's (hue and chroma kept) as little as it takes for every text to.
    static func tint(_ mark: RGBA, _ tokens: ThemeTokens) -> RGBA {
        guard let card = backdrop(tokens) else { return mark }
        let reads = { (tint: RGBA) -> Bool in
            let fill = tint.withAlpha(peak).over(card)
            return text.allSatisfy { name, min in
                tokens.rgba(named: name).map { RGBA.contrast($0.over(fill), fill) >= min } ?? true
            }
        }
        if reads(mark) { return mark }
        let (lightness, chroma, hue) = mark.oklch
        let toward = card.oklch.l
        let at = { (t: Double) in RGBA.oklch(lightness + (toward - lightness) * t, chroma, hue) }
        // Contrast grows as the tint nears the card's lightness, so bisect for the least move.
        var (lo, hi) = (0.0, 1.0)
        for _ in 0..<16 {
            let mid = (lo + hi) / 2
            if reads(at(mid)) { hi = mid } else { lo = mid }
        }
        return at(hi)
    }

    /// `color` as it is, unless it is hard to see on the window or the sidebar: then moved along its lightness just far
    /// enough to be seen on both, so a colour of a person's own, black in a dark theme, is still a mark.
    static func legible(_ color: RGBA, _ tokens: ThemeTokens) -> RGBA {
        ["surface.window", "surface.panel"].compactMap(tokens.rgba(named:))
            .reduce(color) { $0.readable(on: $1.withAlpha(1), ratio: markContrast) }.withAlpha(1)
    }

    /// The hue of a name and the lightness and chroma of the theme's own palette (ANSI 1 to 6, red to cyan), then made
    /// legible.
    static func swatch(hue: Double, grey: Bool = false, _ tokens: ThemeTokens) -> RGBA {
        let colours = tokens.terminal.ansi[1...6].map(\.oklch)
        let n = Double(colours.count)
        let lightness = colours.map(\.l).reduce(0, +) / n
        let chroma = max(colours.map(\.chroma).reduce(0, +) / n, chromaMin)
        return legible(.oklch(lightness, grey ? 0 : chroma, hue), tokens)
    }
}

extension GroupColor {
    /// Where on the colour wheel (OKLCH hue, degrees) the name is.
    var hue: Double {
        switch self {
        case .grey: 0  // unused: grey has no chroma
        case .red: 25
        case .orange: 55
        case .yellow: 95
        case .green: 145
        case .cyan: 190
        case .blue: 255
        case .purple: 305
        case .pink: 350
        }
    }

    /// The colour as the head's dot and the menu show it.
    func mark(_ tokens: ThemeTokens) -> RGBA { GroupWash.swatch(hue: hue, grey: self == .grey, tokens) }

    /// The colour its card is washed with.
    func wash(_ tokens: ThemeTokens) -> RGBA { GroupWash.tint(mark(tokens), tokens) }
}
