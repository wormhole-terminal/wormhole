// The core's snapshots as this window's values: small functions, one per record, each a rename and a conversion (ids to
// `Int`, counts to `Int`, the colour's name to `GroupColor`, the agent's command to `AgentID`). Names the two modules share
// are written with their module: `WormholeBindings.X` is the core's record, `WorkspaceModel.X` the view's.

import Foundation
import WorkspaceModel
import WormholeBindings

extension WorkspaceModel.SidebarSnapshot {
    init(_ core: WormholeBindings.SidebarSnapshot) {
        self.init(
            sections: core.sections.map(WorkspaceModel.SidebarSection.init), selected: core.selected.map(Int.init),
            refreshIn: core.refreshInMs.map { .milliseconds($0) })
    }
}

extension WorkspaceModel.SidebarSection {
    init(_ core: WormholeBindings.SidebarSection) {
        self.init(
            id: core.key, head: core.head.map(WorkspaceModel.GroupHead.init), rows: core.rows.map(WorkspaceModel.SidebarRow.init),
            terminalCount: Int(core.terminalCount), alertCount: Int(core.alertCount))
    }
}

extension WorkspaceModel.GroupHead {
    init(_ core: WormholeBindings.GroupHead) {
        // An empty colour is a group that has none.
        self.init(
            id: core.id, name: core.name, color: GroupColor(key: core.color), isPinned: core.pinned, isFolded: core.folded,
            folder: core.folder, spoken: core.spoken)
    }
}

extension WorkspaceModel.SidebarRow {
    init(_ core: WormholeBindings.SidebarRow) {
        self.init(
            terminal: TerminalSummary(core), number: core.number.map(Int.init), keycap: core.keycap,
            splitMarks: SplitMarks(core.split), secondLine: WorkspaceModel.SecondLine(core.secondLine),
            position: Int(core.position), of: Int(core.of), spoken: core.spoken)
    }
}

extension TerminalSummary {
    init(_ core: WormholeBindings.SidebarRow) {
        self.init(
            id: Int(core.id), title: core.name, folder: core.folder, branch: core.branch, activity: Activity(core.state),
            isPinned: core.pinned, unread: core.unread, hasBell: core.bell, agent: core.agent.flatMap { AgentID(command: $0.key) },
            message: core.message, splitID: core.split.id.map(Int.init), noteCount: Int(core.notes),
            changeCount: Int(core.changes ?? 0))
    }
}

extension Activity {
    init(_ core: RowState) {
        switch core {
        case .needsYou: self = .needsInput
        case .working: self = .working
        case .finished: self = .finished
        case .failed: self = .failed
        case .idle: self = .idle
        case .exited: self = .exited
        }
    }
}

extension SplitMarks {
    init(_ core: SplitInfo) {
        let bracket: WorkspaceModel.Bracket? = switch core.bracket {
        case .start?: .start
        case .mid?: .mid
        case .end?: .end
        case nil: nil
        }
        self.init(bracket: bracket, chain: core.chain, onScreen: core.onScreen, members: Int(core.members))
    }
}

extension WorkspaceModel.SecondLine {
    init(_ core: WormholeBindings.SecondLine) {
        let tone: SecondLineTone = switch core.tone {
        case .plain: .plain
        case .attention: .attention
        case .done: .done
        case .failed: .failed
        }
        self.init(text: core.text ?? "", tone: tone, isCommand: core.mono, trail: core.trail)
    }
}
