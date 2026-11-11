// What the title row has, in order, before the fit: the parts each end draws with their chevrons, and the folder's ways of
// being drawn. The core says all of it (`TitleSnapshot`: which parts the model has something for, where the chevrons go, the
// path as it can be shortened); `sample` is the same for the sample window, which has no core, from the facts it has.

import Foundation
import WorkspaceModel

public struct TitleArrangement: Sendable, Equatable {
    public var left: TitleEnd
    public var right: TitleEnd
    /// The folder's ways of being drawn, widest first (empty without a folder).
    public var paths: [PathText]

    public init(left: TitleEnd, right: TitleEnd, paths: [PathText] = []) { (self.left, self.right, self.paths) = (left, right, paths) }

    public var ends: (left: TitleEnd, right: TitleEnd) { (left, right) }

    public func draws(_ part: TitlePart) -> Bool { left.parts.contains(part) || right.parts.contains(part) }
}

extension TitleArrangement {
    /// The core's row: a slot that has nothing to say is left out, a chevron stands where the core put it. The model has no
    /// agent gauge, ports or pull request (`present` is always false for them): the shell's `facts` say when it has one.
    public init(_ state: TitleState, facts: TitleFacts? = nil) {
        func end(_ slots: [WorkspaceModel.TitleSlot]) -> TitleEnd {
            let drawn = slots.filter { $0.present || TitlePart(rawValue: $0.part).flatMap { part in facts.map { Self.told($0, part) } } == true }.compactMap { slot in TitlePart(rawValue: slot.part).map { ($0, slot.chevronBefore) } }
            return TitleEnd(drawn.map(\.0), chevrons: drawn.map(\.1))
        }
        self.init(
            left: end(state.left), right: end(state.right),
            paths: state.pathSteps.map { PathText(parents: $0.parents, last: $0.last) })
    }

    /// The sample's row from the facts it has: the parts of the person's arrangement that have something to say.
    public static func sample(_ facts: TitleFacts) -> TitleArrangement {
        func has(_ part: TitlePart) -> Bool {
            switch part {
            case .folder, .openIn: facts.folder != nil
            case .branch: facts.branch != nil
            case .branchStatus: facts.branchStatus != nil
            case .terminalStatus: facts.words != nil
            case .agentGauge, .ports, .pullRequest: told(facts, part)
            case .terminal, .splits, .space: true
            }
        }
        func end(_ parts: [TitlePart]) -> TitleEnd {
            let drawn = parts.filter(has)
            return TitleEnd(drawn, chevrons: chevrons(for: drawn))
        }
        let (left, right) = facts.row.ends
        let paths = facts.shownFolder.map { [PathText(parents: "", last: $0)] } ?? []
        return TitleArrangement(left: end(left), right: end(right), paths: paths)
    }

    /// The parts only the shell knows of: what the agent's status line said, the ports its terminal listens on, its pull request.
    private static func told(_ facts: TitleFacts, _ part: TitlePart) -> Bool {
        switch part {
        case .agentGauge: facts.gauge != nil
        case .ports: !facts.ports.isEmpty
        case .pullRequest: facts.pullRequest != nil
        default: false
        }
    }
}
