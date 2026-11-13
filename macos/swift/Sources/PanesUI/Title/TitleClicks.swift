// What a click on a part of the title row that leaves the app does, one function each, shared by the views and the control
// folder (a script cannot press a SwiftUI popover's or menu's button, so it calls these). Each goes through `ExternalOpen`, which
// a test instance records instead of starting a browser.

import DesignSystem
import PanesModel
import WorkspaceModel

@MainActor
public enum TitleClicks {
    public static func openPort(_ port: UInt16) { Ports.url(port).map { ExternalOpen.open($0) } }

    public static func openPullRequest(_ pr: PullRequestFact) { pr.url.map { ExternalOpen.open($0) } }
}

/// The path card's three buttons, as data: what each says, whether it is on and what it does.
public enum PathCardItem: CaseIterable, Sendable {
    case openInFinder, copyPath, insertIntoTerminal

    public var title: String {
        switch self {
        case .openInFinder: PanesText.openInFinder
        case .copyPath: PanesText.copyPath
        case .insertIntoTerminal: PanesText.insertIntoTerminal
        }
    }

    var symbol: String {
        switch self {
        case .openInFinder: "arrow.up.forward.app"
        case .copyPath: "doc.on.doc"
        case .insertIntoTerminal: "text.insert"
        }
    }

    /// The item as the card's button; Copy path says "Copied" for a moment.
    @MainActor func button(_ actions: TitleActions) -> PathCardButton {
        PathCardButton(id: "\(self)", symbol: symbol, title: title, enabled: isOn(actions), copied: self == .copyPath ? PanesText.copied : nil) { perform(actions) }
    }

    public func isOn(_ actions: TitleActions) -> Bool { self != .insertIntoTerminal || actions.canInsert }

    public func perform(_ actions: TitleActions) {
        switch self {
        case .openInFinder: actions.revealFolder()
        case .copyPath: actions.copyPath()
        case .insertIntoTerminal: actions.insertPath()
        }
    }
}
