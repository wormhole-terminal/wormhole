// One pair, measured. The maths is WCAG's (relative luminance of sRGB, ratio of the lighter over the darker with
// 0.05 added to both) and the composition is core's: a translucent colour is laid over its underlay, that over
// `surface.window`, that over black, so a pair is judged on what the screen shows.

import DesignSystem
import Foundation

public struct ContrastCheck: Sendable, Equatable {
    public let pair: ContrastPair
    public let ratio: Double

    public init(pair: ContrastPair, ratio: Double) { (self.pair, self.ratio) = (pair, ratio) }

    /// A pair as core measured it.
    public init(fg: String, bg: String, underlay: String, min: Double, ratio: Double) {
        self.init(pair: ContrastPair(fg, bg, min, underlay: underlay), ratio: ratio)
    }

    public var passes: Bool { ratio >= pair.min }
    public var fg: String { pair.fg }
    public var bg: String { pair.bg }
    public var min: Double { pair.min }

    public static func ratio(_ a: RGBA, _ b: RGBA) -> Double {
        let (la, lb) = (luminance(a), luminance(b))
        return (Swift.max(la, lb) + 0.05) / (Swift.min(la, lb) + 0.05)
    }

    static func luminance(_ c: RGBA) -> Double {
        func linear(_ v: Double) -> Double { v <= 0.04045 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4) }
        return 0.2126 * linear(c.red) + 0.7152 * linear(c.green) + 0.0722 * linear(c.blue)
    }

    /// `pair` in `tokens`; `nil` when the theme lacks one of its tokens.
    public static func measure(_ pair: ContrastPair, in tokens: ThemeTokens) -> ContrastCheck? {
        guard let window = tokens.colorValue(of: "surface.window"),
            let under = tokens.colorValue(of: pair.underlay), let bg = tokens.colorValue(of: pair.bg),
            let fg = tokens.colorValue(of: pair.fg)
        else { return nil }
        let floor = window.over(RGBA(0, 0, 0))
        let background = bg.over(under.over(floor))
        return ContrastCheck(pair: pair, ratio: ratio(fg.over(background), background))
    }

    /// Every pair, measured.
    public static func all(in tokens: ThemeTokens, pairs: [ContrastPair] = ContrastPair.all) -> [ContrastCheck] {
        pairs.compactMap { measure($0, in: tokens) }
    }

    /// Every pair `token` is part of, as the text or as the background (the editor's Contrast section).
    public static func checks(of token: String, in tokens: ThemeTokens) -> [ContrastCheck] {
        all(in: tokens).filter { $0.fg == token || $0.bg == token }
    }

    /// The tokens whose text falls short somewhere: the list marks them and filters by them.
    public static func lowTokens(in tokens: ThemeTokens) -> Set<String> {
        Set(all(in: tokens).filter { !$0.passes }.map(\.fg))
    }
}
