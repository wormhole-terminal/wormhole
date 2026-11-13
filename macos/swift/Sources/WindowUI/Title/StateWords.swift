// The words that say how a terminal is doing after its name: "working · 4m", "done · 3m", "failed · exit 2",
// "needs input", "exited" (GPUI `crumb_words`). The time is the core's (`Waited`, whole minutes); the exit code is the
// failed command's, when the shell gave one.

import WordsModel
import WorkspaceModel

enum StateWords {
    /// "failed · exit 2".
    static func failedExit(_ code: Int32) -> String {
        Words.text("window-state-failed-exit", ["code": .text(String(code))], fallback: "failed · exit \(code)")
    }

    /// Nothing while the terminal is idle. `waited` is how long it has been in its state, nil when not known.
    static func text(_ terminal: TerminalSummary, waited: Waited?) -> String? {
        guard let base = terminal.activity.stateWords else { return nil }
        switch terminal.activity {
        case .failed:
            if let code = terminal.failureCode { return failedExit(code) }
            return waited.map { base + " · " + $0.words } ?? base
        case .working, .finished:
            return waited.map { base + " · " + $0.words } ?? base
        case .idle, .needsInput, .exited:
            return base
        }
    }
}
