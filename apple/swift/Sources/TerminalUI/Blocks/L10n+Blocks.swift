import Foundation

extension L10n {
    static func blockExit(_ code: Int) -> String { tr("terminal-block-exit", "exit \(code)") }
    static var blockCopyOutput: String { tr("terminal-menu-copy-output", "Copy Output") }
    static var blockCopyCommand: String { tr("terminal-menu-copy-command", "Copy Command") }
    static var blockPutAtPrompt: String { tr("terminal-menu-put-command-at-prompt", "Put Command at Prompt") }
    static var blockSelectOutput: String { tr("terminal-menu-select-output", "Select Output") }
    static var blockSpokenPrompt: String { tr("spoken-block-prompt", "prompt") }
    static var blockSpokenRunning: String { tr("spoken-block-running", "running") }
    static var blockSpokenFinished: String { tr("spoken-block-finished", "finished") }
    static var blockSpokenEmpty: String { tr("spoken-block-empty", "empty prompt") }
    static func blockSpokenFailed(_ code: Int) -> String { tr("spoken-block-failed", "failed with exit code \(code)") }
}
