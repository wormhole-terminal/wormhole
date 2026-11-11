import Foundation

extension L10n {
    static var findPlaceholder: String { tr("terminal-find-placeholder", "Find") }
    static var findInvalid: String { tr("terminal-find-invalid", "Not a valid pattern") }
    static var findNoResults: String { tr("terminal-find-no-results", "No results") }
    static func findCount(_ at: String, _ total: String) -> String { tr("terminal-find-count", ["at": .text(at), "total": .text(total)], "\(at) of \(total)") }
    static var findPrevious: String { tr("terminal-find-previous", "Previous match (⇧⌘G)") }
    static var findNext: String { tr("terminal-find-next", "Next match (⌘G)") }
    static var findClose: String { tr("terminal-find-close", "Close find (esc)") }
    static var findMatchCase: String { tr("terminal-find-match-case", "Match case") }
    static var findWholeWord: String { tr("terminal-find-whole-word", "Whole word") }
    static var findRegex: String { tr("terminal-find-regex", "Regular expression") }
    static func findOption(_ option: FindOption) -> String {
        switch option {
        case .matchCase: findMatchCase
        case .wholeWord: findWholeWord
        case .regex: findRegex
        }
    }
}
