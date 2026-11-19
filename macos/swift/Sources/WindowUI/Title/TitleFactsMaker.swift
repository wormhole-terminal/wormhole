// The title row's facts for the selected terminal, gathered from the summary the window already has. The ports and the
// pull request are what the core looked up (`TitleState`); the row hides a part with nothing to say.

import Foundation
import PanesModel
import WordsModel
import WorkspaceModel

enum TitleFactsMaker {
    static func facts(
        for terminal: TerminalSummary, row: TitleRow, statusBarOn: Bool, gauge: GaugeFact? = nil, extras: TitleState? = nil,
        waited: Waited? = nil, home: String = HomeFolder.path
    ) -> TitleFacts {
        let shown = terminal.folder
        let path = shown.map { HomeFolder.expanding($0) }
        return TitleFacts(
            terminalID: terminal.id, folder: path, shownFolder: shown.map { abbreviated($0, home: home) },
            activity: terminal.activity, agent: terminal.agent, label: terminal.title,
            // The status bar says it when it is on (`TitlePart.terminalStatus`).
            words: statusBarOn ? nil : StateWords.text(terminal, waited: waited),
            branch: branch(of: terminal, extras: extras), superproject: extras?.terminal?.superproject,
            branchStatus: branchStatus(of: terminal), gauge: gauge, ports: extras?.ports ?? [],
            pullRequest: extras?.pullRequest, row: row, hiddenFromAgents: terminal.isAgentHidden)
    }

    /// The branch; `.some(nil)` is a repository with no branch (a detached HEAD, as the core's title terminal says), nil is
    /// no repository.
    private static func branch(of terminal: TerminalSummary, extras: TitleState?) -> String?? {
        if let name = terminal.branch { return .some(name) }
        return (extras?.terminal?.detached ?? terminal.detached) ? .some(nil) : nil
    }

    /// What git says of the branch (the core's feed): an unclean tree and how far it is from its upstream; nothing to say
    /// draws nothing. Without a branch there is no status.
    private static func branchStatus(of terminal: TerminalSummary) -> BranchStatus? {
        guard terminal.branch != nil, terminal.isDirty || terminal.ahead > 0 || terminal.behind > 0 else { return nil }
        return BranchStatus(dirty: terminal.isDirty, ahead: terminal.ahead, behind: terminal.behind)
    }

    private static func abbreviated(_ path: String, home: String) -> String {
        path.hasPrefix(home) ? "~" + path.dropFirst(home.count) : path
    }
}
