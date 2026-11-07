// The words of the title row and the tab bar (`title-row.ftl` and the sidebar's tab catalog in `wormhole-text`).

import Foundation
import PanesModel

extension PanesText {
    static var openInFinder: String { String(localized: "title-row-path-open-in-finder", defaultValue: "Open in Finder") }
    static var copyPath: String { String(localized: "title-row-path-copy", defaultValue: "Copy path") }
    static var copied: String { String(localized: "title-row-path-copied", defaultValue: "Copied") }
    static var insertIntoTerminal: String { String(localized: "title-row-path-insert", defaultValue: "Insert into terminal") }
    static func openFolder(_ shown: String) -> String { String(localized: "title-row-path-spoken-open", defaultValue: "Open folder \(shown) in Finder") }
    static var detached: String { String(localized: "title-row-detached", defaultValue: "detached") }
    static func branch(_ name: String) -> String { String(localized: "title-row-branch-spoken", defaultValue: "branch \(name)") }
    static func submodule(_ phrase: String) -> String { String(localized: "title-row-submodule", defaultValue: "A submodule: \(phrase)") }
    static var hiddenFromAgents: String { String(localized: "title-row-hidden-from-agents", defaultValue: "Hidden from agents") }
    static var openFolderIn: String { String(localized: "title-row-open-in-header", defaultValue: "Open folder in") }
    static var openInAnother: String { String(localized: "title-row-open-in-another", defaultValue: "Open in another app") }
    static func openFolderInApp(_ app: String) -> String { String(localized: "title-row-open-in-app", defaultValue: "Open folder in \(app)") }
    static func appMissing(_ missing: String, _ app: String) -> String {
        String(localized: "title-row-open-in-missing", defaultValue: "\(missing) isn't installed. Open folder in \(app)")
    }
    static var splitRightKey: String { String(localized: "title-row-split-right", defaultValue: "Split right (⌘D)") }
    static var splitDownKey: String { String(localized: "title-row-split-down", defaultValue: "Split down (⇧⌘D)") }
    static var moreOpenInSplit: String { String(localized: "title-row-more-open-in-split", defaultValue: "Open in, split") }
    static var moreOpenIn: String { String(localized: "title-row-more-open-in", defaultValue: "Open in") }
    static var moreSplit: String { String(localized: "title-row-more-split", defaultValue: "Split") }
    static var showInTitleRow: String { String(localized: "title-row-menu-header", defaultValue: "Show in title row") }
    static var resetTitleRow: String { String(localized: "title-row-menu-reset", defaultValue: "Reset title row") }
    static var customizeTitleRow: String { String(localized: "title-row-menu-customize", defaultValue: "Customize title row…") }

    static func partName(_ part: TitlePart) -> String {
        switch part {
        case .folder: String(localized: "title-row-part-folder", defaultValue: "Folder")
        case .branch: String(localized: "title-row-part-branch", defaultValue: "Branch")
        case .branchStatus: String(localized: "title-row-part-branch-status", defaultValue: "Branch status")
        case .terminal: String(localized: "title-row-part-terminal", defaultValue: "Terminal name")
        case .terminalStatus: String(localized: "title-row-part-terminal-status", defaultValue: "Terminal status")
        case .space: String(localized: "title-row-part-space", defaultValue: "Flexible space")
        case .agentGauge: String(localized: "title-row-part-agent-gauge", defaultValue: "Agent model and context")
        case .ports: String(localized: "title-row-part-ports", defaultValue: "Open ports")
        case .pullRequest: String(localized: "title-row-part-pull-request", defaultValue: "Pull request")
        case .openIn: String(localized: "title-row-part-open-in", defaultValue: "Open in…")
        case .splits: String(localized: "title-row-part-splits", defaultValue: "Split buttons")
        }
    }
}
