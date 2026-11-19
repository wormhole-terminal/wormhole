// What the token list and the editor read from the document: the bands the filter keeps, each row's chip and
// value, where a value comes from, the contrast pairs and what follows a token. Worked out from the resolved
// tokens and core's contrast report, so they are Rust's answers and the same ones the preview draws.

import DesignSystem
import Foundation

extension StudioDocument {
    public func origin(of token: String) -> TokenOrigin {
        if file.own(token, in: shown) != nil { return .yours }
        return file.baseValue(token, in: shown) != nil ? .base : .workedOut
    }

    /// What the base has for the token, if it is a copy: Reset goes back to it.
    public func baseValue(of token: String) -> String? { file.baseValue(token, in: shown) }

    /// Edited since the last save.
    public func isChanged(_ token: String) -> Bool { file.own(token, in: shown) != saved.own(token, in: shown) }

    /// The tokens whose text falls short somewhere, by core's check (the card's own pairs).
    public var lowContrastTokens: Set<String> { contrast.lowTokens }

    public var visibleGroups: [TokenGroup] {
        let low = lowContrastTokens
        let uses = environment.knowledge.uses
        let facts = TokenFilter.Facts(
            origin: { [self] in origin(of: $0) }, low: { low.contains($0) },
            inArea: { name, area in uses.contains { $0.area == area && $0.draws(with: name) } })
        return filter.visible(groups, facts: facts)
    }

    /// The counts the Yours, Base and Low contrast segments carry.
    public var counts: (yours: Int, base: Int, low: Int) {
        let all = groups.flatMap(\.tokens)
        return (all.filter { origin(of: $0) == .yours }.count, all.filter { origin(of: $0) == .base }.count, all.filter(lowContrastTokens.contains).count)
    }

    /// Whether a copy's base is under the shown version (the Base segment is offered then).
    public var hasBase: Bool { file.baseSource(in: shown) != nil }

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

    /// What draws with `token` as core lists it (`uses_of`): "Used by" and a metric's count.
    public func uses(of token: String) -> [TokenUse] { environment.knowledge.uses.filter { $0.draws(with: token) } }

    /// The gallery's components whose views really draw with `token`: what the gallery outlines.
    public func drawn(with token: String) -> [TokenUse] { environment.knowledge.drawn.filter { $0.draws(with: token) } }

    public var contrastIssues: [ContrastIssue] { contrast.issues }

    public func checks(of token: String) -> [ContrastCheck] { contrast.checks(of: token) }

    /// Where a value of the shown version lives, for the chip of a pair.
    public func place(of token: String) -> ThemeFile.Place {
        file.place(of: token, version: shown, group: group(of: token))
    }
}

extension StudioDocument {
    /// What core offers at UTF-8 offset `caret` of `text` typed for `token`: nothing for a metric (a number) or without the core.
    public func completions(_ text: String, caret: Int, for token: String) -> FieldCompletions? {
        guard !token.hasPrefix("metrics.") else { return nil }
        return environment.files?.completions(text, caret: caret, target: token)
    }

    /// The faint word beside a name in the list: its group in the token list.
    public func groupName(of name: String) -> String? { group(of: name) }

    /// Every name an expression may refer to: for completion.
    public var expressionNames: [String] {
        groups.flatMap(\.tokens).filter { !$0.hasPrefix("metrics.") }
    }
}
