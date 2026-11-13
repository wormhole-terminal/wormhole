// The title row's data (core `TitleSnapshot`): the parts on each side with their chevrons, the folder's ways to be drawn
// and the path menu. The row's terminal is the same summary the sidebar uses.

import Foundation
import WorkspaceModel
import WormholeBindings

extension TitleState {
    init(_ core: WormholeBindings.TitleSnapshot) {
        self.init(
            terminal: core.terminal.map(WorkspaceModel.TerminalSummary.init), left: core.left.map(WorkspaceModel.TitleSlot.init),
            right: core.right.map(WorkspaceModel.TitleSlot.init), hidden: core.hidden.map(\.key),
            pathSteps: core.pathSteps.map { PathStepText(parents: $0.parents, last: $0.last) },
            pathMenu: core.pathMenu.map { PathMenuFolder(path: $0.path, name: $0.name) }, ports: core.ports,
            pullRequest: core.pullRequest.map { PullRequestFact($0, words: core.pullRequestWords ?? "") })
    }
}

extension PullRequestFact {
    init(_ core: WormholeBindings.PullRequest, words: String) {
        let state: State = switch core.state {
        case .open: .open
        case .merged: .merged
        case .closed: .closed
        }
        let checks: Checks? = switch core.checks {
        case .pending?: .pending
        case .failing?: .failing
        case .passing?: .passing
        case nil: nil
        }
        self.init(number: Int(core.number), url: URL(string: core.url), state: state, draft: core.draft, checks: checks, words: words)
    }
}

extension WorkspaceModel.TitleSlot {
    init(_ core: WormholeBindings.TitleSlot) {
        self.init(part: core.part.key, present: core.present, chevronBefore: core.chevronBefore)
    }
}

extension WorkspaceModel.TerminalSummary {
    init(_ core: TitleTerminal) {
        self.init(
            id: Int(core.id), title: core.name, folder: core.folder, branch: core.branch, activity: Activity(core.state),
            agent: core.agent.flatMap { AgentID(command: $0.key) }, message: core.message,
            glyph: WorkspaceModel.TileGlyph(core.glyph))
        superproject = core.superproject
        detached = core.detached
    }
}
