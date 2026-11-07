// The words of the title row and the tab bar (`title-row.ftl` and the sidebar's tab catalog in `wormhole-text`).

import Foundation
import PanesModel
import WordsModel

extension PanesText {
    static var openInFinder: String { Words.text("title-row-path-open-in-finder", fallback: "Open in Finder") }
    static var copyPath: String { Words.text("title-row-path-copy", fallback: "Copy path") }
    static var copied: String { Words.text("title-row-path-copied", fallback: "Copied") }
    static var insertIntoTerminal: String { Words.text("title-row-path-insert", fallback: "Insert into terminal") }
    static func openFolder(_ shown: String) -> String { Words.text("title-row-path-spoken-open", fallback: "Open folder \(shown) in Finder") }
    static var detached: String { Words.text("title-row-detached", fallback: "detached") }
    static func branch(_ name: String) -> String { Words.text("title-row-branch-spoken", fallback: "branch \(name)") }
    static func submodule(_ phrase: String) -> String { Words.text("title-row-submodule", fallback: "A submodule: \(phrase)") }
    static var hiddenFromAgents: String { Words.text("title-row-hidden-from-agents", fallback: "Hidden from agents") }
    static var openFolderIn: String { Words.text("title-row-open-in-header", fallback: "Open folder in") }
    static var openInAnother: String { Words.text("title-row-open-in-another", fallback: "Open in another app") }
    static func openFolderInApp(_ app: String) -> String { Words.text("title-row-open-in-app", fallback: "Open folder in \(app)") }
    static func appMissing(_ missing: String, _ app: String) -> String {
        Words.text("title-row-open-in-missing", fallback: "\(missing) isn't installed. Open folder in \(app)")
    }
    static var splitRightKey: String { Words.text("title-row-split-right", fallback: "Split right (⌘D)") }
    static var splitDownKey: String { Words.text("title-row-split-down", fallback: "Split down (⇧⌘D)") }
    static var moreOpenInSplit: String { Words.text("title-row-more-open-in-split", fallback: "Open in, split") }
    static var moreOpenIn: String { Words.text("title-row-more-open-in", fallback: "Open in") }
    static var moreSplit: String { Words.text("title-row-more-split", fallback: "Split") }
    static var showInTitleRow: String { Words.text("title-row-menu-header", fallback: "Show in title row") }
    static var resetTitleRow: String { Words.text("title-row-menu-reset", fallback: "Reset title row") }
    static var customizeTitleRow: String { Words.text("title-row-menu-customize", fallback: "Customize title row…") }

    static func partName(_ part: TitlePart) -> String {
        switch part {
        case .folder: Words.text("title-row-part-folder", fallback: "Folder")
        case .branch: Words.text("title-row-part-branch", fallback: "Branch")
        case .branchStatus: Words.text("title-row-part-branch-status", fallback: "Branch status")
        case .terminal: Words.text("title-row-part-terminal", fallback: "Terminal name")
        case .terminalStatus: Words.text("title-row-part-terminal-status", fallback: "Terminal status")
        case .space: Words.text("title-row-part-space", fallback: "Flexible space")
        case .agentGauge: Words.text("title-row-part-agent-gauge", fallback: "Agent model and context")
        case .ports: Words.text("title-row-part-ports", fallback: "Open ports")
        case .pullRequest: Words.text("title-row-part-pull-request", fallback: "Pull request")
        case .openIn: Words.text("title-row-part-open-in", fallback: "Open in…")
        case .splits: Words.text("title-row-part-splits", fallback: "Split buttons")
        }
    }
}
