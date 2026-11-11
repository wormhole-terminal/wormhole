// Everything the title row draws, worked out once per frame (`plan_title`, `extras_wanted`): what each part
// wants, asked of a text measurer so the rules stay testable without fonts, then core's fit decides
// what the width allows.

import Foundation
import WordsModel

/// The type faces the row measures in: the label role (13) in regular or medium for the path and the name,
/// the caption role (12) for words and chips.
public enum TitleFace: Sendable {
    case label, labelMedium, caption, captionMedium
}

public protocol TextMeasuring {
    func width(_ text: String, _ face: TitleFace) -> Double
}

public struct TitlePlan: Sendable, Equatable {
    public var facts: TitleFacts
    public var ends: (left: TitleEnd, right: TitleEnd)
    public var steps: [PathStep]
    public var fit: RowFit

    public static func == (a: TitlePlan, b: TitlePlan) -> Bool {
        a.facts == b.facts && a.ends.left == b.ends.left && a.ends.right == b.ends.right && a.steps == b.steps && a.fit == b.fit
    }

    /// Whether either end holds `part` (the row's arrangement and the facts allow it; the room may not).
    public func draws(_ part: TitlePart) -> Bool { ends.left.parts.contains(part) || ends.right.parts.contains(part) }

    /// Whether `part` is on screen now: it is held and the row had room for it. Folded, Open in… and the
    /// splits are one `⋯`, which the first button part stands for.
    public func isDrawn(_ part: TitlePart) -> Bool {
        guard draws(part) else { return false }
        switch part {
        case .folder, .terminal: return true
        case .branch: return fit.chip != nil
        case .branchStatus: return fit.extras.shows(.branchStatus)
        case .terminalStatus: return fit.words && facts.words != nil
        case .agentGauge: return fit.extras.shows(.gauge)
        case .ports: return fit.extras.shows(.ports)
        case .pullRequest: return fit.extras.shows(.pullRequest)
        case .openIn, .splits:
            let first = (ends.left.parts + ends.right.parts).first { $0.kind == .button }
            return !fit.folded || first == part
        case .space: return false
        }
    }
}

public enum TitlePlanner {
    /// The plan for `facts` in a row `width` points wide inside its padding.
    /// The row at `width`: the parts and the folder's shapes are `arrangement`'s (the core's), the widths are measured here and
    /// the order in which the row gives way is `fitting`'s (the core's).
    public static func plan(
        _ facts: TitleFacts, _ arrangement: TitleArrangement, width: Double, measure: some TextMeasuring, fitting: any RowFitting
    ) -> TitlePlan {
        let ends = arrangement.ends
        let drawn = arrangement.draws
        let steps = drawn(.folder)
            ? PathSteps.measured(arrangement.paths, lastMax: TitleMetrics.pathLastMax) { face, text in
                measure.width(text, face == .parents ? .label : .labelMedium)
            } : []
        var wants = RowWants()
        wants.path = steps.map { TitleMetrics.crumbGap + $0.width }
        // The status words and, in a submodule, whose it is: they give way together.
        wants.words = [drawn(.terminalStatus) ? facts.words : nil, drawn(.branch) ? facts.superproject : nil]
            .compactMap { $0 }.reduce(0) { $0 + TitleMetrics.crumbGap + measure.width($1, .caption) }
        wants.chip = (drawn(.branch) ? facts.branch : nil).map { branch in
            ChipWidth(
                natural: min(measure.width(branch ?? Words.text("title-row-detached", fallback: "detached"), .captionMedium) + TitleMetrics.chipChrome, TitleMetrics.chipMax),
                icon: TitleMetrics.chipIcon)
        }
        (wants.extras, wants.portsNarrower) = extras(facts, measure)
        // Open in… (which `has` keeps for a terminal with a folder) and the splits, each where it is arranged;
        // folded they are one `⋯`.
        let buttons = [(TitlePart.openIn, TitleMetrics.openInWidth), (.splits, TitleMetrics.splitsWidth)].filter { drawn($0.0) }.map(\.1)
        if !buttons.isEmpty {
            wants.group = TitleMetrics.rowGap + buttons.reduce(0, +)
            wants.folded = TitleMetrics.rowGap + TitleMetrics.moreWidth
        }
        let terminal = drawn(.terminal)
        wants.name = terminal ? min(measure.width(facts.label, .labelMedium), TitleMetrics.nameMax) : 0
        wants.nameMin = terminal ? TitleMetrics.textMin : 0
        wants.fixed = TitleLayout.fixedWidth(ends)
        return TitlePlan(facts: facts, ends: ends, steps: steps, fit: fitting.fit(avail: width, wants))
    }

    /// How wide each extra is with the gap in front of it, and the ports with a chip fewer each time.
    static func extras(_ facts: TitleFacts, _ measure: some TextMeasuring) -> (Extras, [Double]) {
        let text = { (text: String) in measure.width(text, .captionMedium) }
        let chip = { (icon: Bool, text: Double) in
            2 * TitleMetrics.extraPad + text + (icon ? TitleMetrics.extraIcon + TitleMetrics.extraGap : 0)
        }
        var extras = Extras()
        if let status = facts.branchStatus {
            let counts = [status.ahead, status.behind].filter { $0 > 0 }.map { TitleMetrics.extraIcon + text(String($0)) }
            let parts = (status.dirty ? [TitleMetrics.statusDot] : []) + counts
            let gaps = Double(max(parts.count - 1, 0)) * TitleMetrics.extraGap
            extras = extras.with(.branchStatus, TitleMetrics.crumbGap + parts.reduce(0, +) + gaps)
        }
        if let gauge = facts.gauge {
            // The name, then the meter and its share: each a part, with a gap between parts.
            var parts: [Double] = []
            if let model = gauge.model { parts.append(min(text(model), TitleMetrics.gaugeModelMax)) }
            if let percent = gauge.percent { parts += [TitleMetrics.meter, text("\(percent)%")] }
            let gaps = Double(max(parts.count - 1, 0)) * TitleMetrics.extraGap
            extras = extras.with(.gauge, TitleMetrics.chipGap + 2 * TitleMetrics.extraPad + parts.reduce(0, +) + gaps)
        }
        var narrower: [Double] = []
        if !facts.ports.isEmpty {
            // The ports as `chips` chips of their own and a "+n" for the others.
            let ports = { (chips: Int) -> Double in
                let own = facts.ports.prefix(chips).reduce(0) { $0 + TitleMetrics.chipGap + chip(true, text(Ports.label($1))) }
                let rest = facts.ports.count - chips
                return own + (rest > 0 ? TitleMetrics.chipGap + chip(false, text("+\(rest)")) : 0)
            }
            let shown = Ports.chips(facts.ports).shown.count
            extras = extras.with(.ports, ports(shown))
            // A chip fewer at a time, down to the one.
            narrower = shown > 1 ? (1..<shown).reversed().map(ports) : []
        }
        if let pr = facts.pullRequest {
            let checks = pr.openChecks != nil ? TitleMetrics.extraGap + TitleMetrics.extraIcon : 0
            extras = extras.with(.pullRequest, TitleMetrics.chipGap + chip(true, text(pr.label)) + checks)
        }
        return (extras, narrower)
    }
}
