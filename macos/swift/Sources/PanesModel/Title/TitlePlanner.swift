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
    /// The most the agent gauge's model name may take: what the row can spare, at least its design cap.
    public var modelMax: Double

    public static func == (a: TitlePlan, b: TitlePlan) -> Bool {
        a.facts == b.facts && a.ends.left == b.ends.left && a.ends.right == b.ends.right && a.steps == b.steps && a.fit == b.fit && a.modelMax == b.modelMax
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

/// The most a text that can be cut takes of the row before the other parts give way (`PATH_LAST_MAX`, `BRANCH_CHIP_MAX`,
/// `TITLE_TAB_MAX_W`, the model's widest in `style/extras.rs`): a long branch or title may not send the path away. They are
/// what the parts are guaranteed against each other, not what a roomy row shows: room none of the parts needs goes to them
/// (`TitlePlanner.plan`), so a text is cut only when the row really has no more.
struct TitleCaps: Equatable {
    var pathLast: Double
    var name: Double
    var chip: Double
    var model: Double

    static let design = TitleCaps(
        pathLast: TitleMetrics.pathLastMax, name: TitleMetrics.nameMax, chip: TitleMetrics.chipMax, model: TitleMetrics.gaugeModelMax)
    static let unbounded = TitleCaps(pathLast: .infinity, name: .infinity, chip: .infinity, model: .infinity)
}

/// What the row wants under some caps: the path's steps measured, each part's width, and the widths of the texts the caps cut.
private struct TitleSketch {
    var steps: [PathStep]
    var wants: RowWants
    var caps: TitleCaps
    /// The last folder, the name, the branch chip and the model as drawn under `caps`.
    var lastWidth: Double
    var nameWidth: Double
    var chipWidth: Double
    var modelWidth: Double

    init(_ facts: TitleFacts, _ arrangement: TitleArrangement, caps: TitleCaps, measure: some TextMeasuring) {
        let drawn = arrangement.draws
        steps = drawn(.folder)
            ? PathSteps.measured(arrangement.paths, lastMax: caps.pathLast) { face, text in
                measure.width(text, face == .parents ? .label : .labelMedium)
            } : []
        var wants = RowWants()
        wants.path = steps.map { TitleMetrics.crumbGap + $0.width }
        // The status words and, in a submodule, whose it is: they give way together.
        wants.words = [drawn(.terminalStatus) ? facts.words : nil, drawn(.branch) ? facts.superproject : nil]
            .compactMap { $0 }.reduce(0) { $0 + TitleMetrics.crumbGap + measure.width($1, .caption) }
        var chip = 0.0
        wants.chip = (drawn(.branch) ? facts.branch : nil).map { branch in
            let natural = measure.width(branch ?? Words.text("title-row-detached", fallback: "detached"), .captionMedium) + TitleMetrics.chipChrome
            chip = min(natural, caps.chip)
            return ChipWidth(natural: chip, icon: TitleMetrics.chipIcon)
        }
        chipWidth = chip
        modelWidth = facts.gauge?.model.map { min(measure.width($0, .captionMedium), caps.model) } ?? 0
        (wants.extras, wants.portsNarrower) = TitlePlanner.extras(facts, measure, modelWidth: modelWidth)
        // Open in… (which `has` keeps for a terminal with a folder) and the splits, each where it is arranged;
        // folded they are one `⋯`.
        let buttons = [(TitlePart.openIn, TitleMetrics.openInWidth), (.splits, TitleMetrics.splitsWidth)].filter { drawn($0.0) }.map(\.1)
        if !buttons.isEmpty {
            wants.group = TitleMetrics.rowGap + buttons.reduce(0, +)
            wants.folded = TitleMetrics.rowGap + TitleMetrics.moreWidth
        }
        let terminal = drawn(.terminal)
        nameWidth = terminal ? min(measure.width(facts.label, .labelMedium), caps.name) : 0
        wants.name = nameWidth
        wants.nameMin = terminal ? TitleMetrics.textMin : 0
        wants.fixed = TitleLayout.fixedWidth(arrangement.ends)
        lastWidth = steps.first.map { measure.width($0.text.last, .labelMedium) } ?? 0
        self.wants = wants
        self.caps = caps
    }

    /// The room `fit` leaves over in a row `width` wide (what `fit_row` counts as the row's rest, and the name).
    func spare(after fit: RowFit, width: Double) -> Double {
        let path: Double = if case .step(let ix) = fit.path { wants.path[ix] } else { 0 }
        let used = wants.fixed + path + (fit.words ? wants.words : 0) + fit.extras.total + (fit.chip ?? 0)
            + (fit.folded ? wants.folded : wants.group) + fit.name
        return width - used
    }
}

public enum TitlePlanner {
    /// The plan for `facts` in a row `width` points wide inside its padding: the parts and the folder's shapes are
    /// `arrangement`'s (the core's), the widths are measured here and the order in which the row gives way is `fitting`'s
    /// (the core's). The texts that can be cut are first held to their caps so the order holds; whatever the row has left
    /// then goes to them, and a text is cut only where there is no more room.
    public static func plan(
        _ facts: TitleFacts, _ arrangement: TitleArrangement, width: Double, measure: some TextMeasuring, fitting: any RowFitting
    ) -> TitlePlan {
        let held = TitleSketch(facts, arrangement, caps: .design, measure: measure)
        let heldFit = fitting.fit(avail: width, held.wants)
        var plan = TitlePlan(facts: facts, ends: arrangement.ends, steps: held.steps, fit: heldFit, modelMax: held.caps.model)
        var spare = held.spare(after: heldFit, width: width) - TitleMetrics.planSlack
        guard spare > 0 else { return plan }
        // Each text that was cut takes what it lacks, the path's folder first, until the room is gone.
        let whole = TitleSketch(facts, arrangement, caps: .unbounded, measure: measure)
        var caps = held.caps
        func grow(_ cap: inout Double, from had: Double, to whole: Double) {
            let give = min(max(whole - had, 0), spare)
            cap = had + give
            spare -= give
        }
        if case .step = heldFit.path { grow(&caps.pathLast, from: held.lastWidth, to: whole.lastWidth) }
        grow(&caps.name, from: held.nameWidth, to: whole.nameWidth)
        if heldFit.chip != nil { grow(&caps.chip, from: held.chipWidth, to: whole.chipWidth) }
        if heldFit.extras.shows(.gauge) { grow(&caps.model, from: held.modelWidth, to: whole.modelWidth) }
        guard caps != held.caps else { return plan }
        let grown = TitleSketch(facts, arrangement, caps: caps, measure: measure)
        let grownFit = fitting.fit(avail: width, grown.wants)
        // The room was counted to leave every part as it was; a fit that decides otherwise keeps the held plan.
        guard grownFit.path == heldFit.path, grownFit.words == heldFit.words, grownFit.extras == heldFit.extras,
            grownFit.portsStep == heldFit.portsStep, grownFit.folded == heldFit.folded
        else { return plan }
        plan = TitlePlan(facts: facts, ends: arrangement.ends, steps: grown.steps, fit: grownFit, modelMax: caps.model)
        return plan
    }

    /// How wide each extra is with the gap in front of it, and the ports with a chip fewer each time.
    static func extras(_ facts: TitleFacts, _ measure: some TextMeasuring, modelWidth: Double) -> (Extras, [Double]) {
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
            if gauge.model != nil { parts.append(modelWidth) }
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
