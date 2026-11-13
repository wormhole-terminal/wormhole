// The status bar's counts and its detail asked of the core (`statusCounts`, `statusDetail`: `view::layout::status_bar`).

import WorkspaceModel
import WormholeBindings

public struct CoreStatusRules: StatusRuling {
    public init() {}

    public func counts(of terminals: [WorkspaceModel.TerminalSummary]) -> StatusCounts {
        let entries = terminals.map { StatusEntryRecord(kind: Self.kind($0.activity), sinceMs: $0.since) }
        let counted = statusCounts(entries: entries)
        return StatusCounts(
            needsYou: counted.needsYou.map(Int.init),
            rest: counted.rest.map { StatusCounts.Item(activity: Self.activity($0.kind), count: Int($0.count), terminal: Int($0.terminal)) })
    }

    public func detail(windowWidth: Double) -> WorkspaceModel.StatusBarDetail {
        switch statusDetail(windowWidth: Float(windowWidth)) {
        case .lean: .lean
        case .numbers: .numbers
        case .words: .words
        }
    }

    private static func kind(_ activity: Activity) -> StatusCountKind {
        switch activity {
        case .working: .working
        case .needsInput: .needsYou
        case .finished: .done
        case .failed: .failed
        case .idle, .exited: .quiet
        }
    }

    private static func activity(_ kind: StatusCountKind) -> Activity {
        switch kind {
        case .working: .working
        case .needsYou: .needsInput
        case .done: .finished
        case .failed: .failed
        case .quiet: .idle
        }
    }
}
