// One text-on-background pair as core measured it (`contrast_checks`: core's pair list, WCAG's maths, a translucent
// colour laid over its underlay, that over `surface.window`, that over black), and the whole theme's answer: every
// pair and the ones that fall short, from one resolve, so the list's marks, the Low filter, the editor's Contrast
// section and the card read the same thing. Swift keeps no pair table and no ratio maths of its own.

import Foundation

public struct ContrastCheck: Sendable, Equatable {
    public let fg: String
    public let bg: String
    /// What a translucent `bg` is judged over.
    public let underlay: String
    public let min: Double
    public let ratio: Double

    public init(fg: String, bg: String, underlay: String, min: Double, ratio: Double) {
        (self.fg, self.bg, self.underlay, self.min, self.ratio) = (fg, bg, underlay, min, ratio)
    }

    public var passes: Bool { ratio >= min }
}

public struct ContrastReport: Sendable, Equatable {
    /// Every pair the theme has the tokens for, in core's order.
    public var checks: [ContrastCheck]
    /// The pairs that fall short, in the same order, each with core's fix.
    public var issues: [ContrastIssue]

    public init(checks: [ContrastCheck] = [], issues: [ContrastIssue] = []) { (self.checks, self.issues) = (checks, issues) }

    /// The tokens whose text falls short somewhere: the list marks them and filters by them.
    public var lowTokens: Set<String> { Set(issues.map(\.check.fg)) }

    /// Every pair `token` is part of, as the text or as the background (the editor's Contrast section).
    public func checks(of token: String) -> [ContrastCheck] { checks.filter { $0.fg == token || $0.bg == token } }
}
