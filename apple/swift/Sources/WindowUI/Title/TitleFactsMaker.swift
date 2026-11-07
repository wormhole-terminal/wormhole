// The title row's facts for the selected terminal, gathered from the summary the window already has. What needs
// git, ports, a pull request or the agent's gauge comes with the Rust core (`window/title/breadcrumb.rs`); the
// row hides a part with nothing to say.

import Foundation
import PanesModel
import WorkspaceModel

enum TitleFactsMaker {
    static func facts(for terminal: TerminalSummary, row: TitleRow, statusBarOn: Bool, home: String = NSHomeDirectory()) -> TitleFacts {
        let shown = terminal.folder
        let path = shown.map { ($0 as NSString).expandingTildeInPath }
        return TitleFacts(
            terminalID: terminal.id, folder: path, shownFolder: shown.map { abbreviated($0, home: home) },
            activity: terminal.activity, agent: terminal.agent, label: terminal.title,
            // The status bar says it when it is on (`TitlePart.terminalStatus`).
            words: statusBarOn ? nil : terminal.activity.words,
            branch: terminal.branch.map { .some($0) }, branchStatus: terminal.changeCount > 0 ? BranchStatus(dirty: true) : nil,
            row: row)
    }

    private static func abbreviated(_ path: String, home: String) -> String {
        path.hasPrefix(home) ? "~" + path.dropFirst(home.count) : path
    }
}

private extension Activity {
    var words: String? {
        switch self {
        case .idle: nil
        case .working: "working"
        case .needsInput: "needs input"
        case .finished: "finished"
        case .failed: "failed"
        case .exited: "exited"
        }
    }
}
