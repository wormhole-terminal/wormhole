// Every terminal, folded ones too, and the counts the status bar shows (core `TerminalsSnapshot`).

import WorkspaceModel
import WormholeBindings

extension WorkspaceModel.TerminalsSnapshot {
    init(_ core: WormholeBindings.TerminalsSnapshot) {
        self.init(
            terminals: core.terminals.map(WorkspaceModel.TerminalSummary.init),
            counts: WorkspaceModel.TerminalCounts(
                needsYou: Int(core.counts.needsYou), working: Int(core.counts.working), finished: Int(core.counts.finished),
                failed: Int(core.counts.failed), quiet: Int(core.counts.quiet)))
    }
}

extension WorkspaceModel.TerminalSummary {
    /// A terminal of the all-terminals list: no notes or change counts (those are the rows'), but its place in a group.
    init(_ core: WormholeBindings.TerminalSummary) {
        self.init(
            id: Int(core.id), title: core.name, folder: core.folder, branch: core.branch, activity: Activity(core.state),
            isPinned: core.pinned, unread: core.unread, hasBell: core.bell, agent: core.agent.flatMap { AgentID(command: $0.key) },
            message: core.message, splitID: core.split.map(Int.init), isAgentHidden: core.agentHidden, forgetsText: core.forgetText,
            group: core.group, groupName: core.groupName, isFolded: core.folded, glyph: WorkspaceModel.TileGlyph(core.glyph))
    }
}
