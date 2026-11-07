// What the token list and the editor read from the document: the bands the filter keeps, each row's chip and
// value, where a value comes from, the contrast pairs and what follows a token. Worked out from the resolved
// tokens, so they are Rust's answers and the same ones the preview draws.

import DesignSystem
import Foundation

extension StudioDocument {
    public func origin(of token: String) -> TokenOrigin { file.own(token, in: shown) != nil ? .yours : .workedOut }

    /// Edited since the last save.
    public func isChanged(_ token: String) -> Bool { file.own(token, in: shown) != saved.own(token, in: shown) }

    public var lowContrastTokens: Set<String> { ContrastCheck.lowTokens(in: tokens) }

    public var visibleGroups: [TokenGroup] {
        let low = lowContrastTokens
        let uses = environment.knowledge.uses
        let facts = TokenFilter.Facts(
            origin: { [self] in origin(of: $0) }, low: { low.contains($0) },
            inArea: { name, area in uses.contains { $0.area == area && $0.draws(with: name) } })
        return filter.visible(groups, facts: facts)
    }

    /// The counts the Yours and Low contrast segments carry.
    public var counts: (yours: Int, low: Int) {
        let all = groups.flatMap(\.tokens)
        return (all.filter { origin(of: $0) == .yours }.count, all.filter(lowContrastTokens.contains).count)
    }

    public func row(for token: String) -> TokenRowValue {
        if let pixels = tokens.metric(named: token) {
            return TokenRowValue(chip: .metric(pixels), text: "\(NumberText.pixels(pixels))px")
        }
        let color = resolvedColor(token)
        return TokenRowValue(chip: .color(color), text: file.own(token, in: shown) ?? color.hex)
    }

    /// The colour a token works out to, magenta for a name the theme lacks (as everywhere).
    public func resolvedColor(_ token: String) -> RGBA { tokens.colorValue(of: token) ?? ThemeTokens.missing }

    public func rule(for token: String) -> String? { environment.knowledge.rule(for: token, appearance: appearance) }

    public func followers(of token: String) -> [String] {
        environment.knowledge.followers(of: token, among: groups.flatMap(\.tokens), appearance: appearance)
    }

    public func description(of token: String) -> String? {
        environment.library.catalog.first { $0.name == token }?.description
    }

    public func uses(of token: String) -> [TokenUse] { environment.knowledge.uses.filter { $0.draws(with: token) } }

    public var contrastIssues: [ContrastIssue] {
        ContrastCheck.issues(in: tokens) { [self] in file.own($0, in: shown) ?? rule(for: $0) }
    }

    public func checks(of token: String) -> [ContrastCheck] { ContrastCheck.checks(of: token, in: tokens) }

    /// Where a value of the shown version lives, for the chip of a pair.
    public func place(of token: String) -> ThemeFile.Place {
        file.place(of: token, version: shown, group: group(of: token))
    }
}

extension StudioDocument {
    /// Every name an expression may refer to: for completion.
    public var expressionNames: [String] {
        groups.flatMap(\.tokens).filter { !$0.hasPrefix("metrics.") }
    }
}
