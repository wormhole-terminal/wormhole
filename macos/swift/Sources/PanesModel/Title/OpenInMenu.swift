// The Open in… button and the list behind its chevron (and the ⋯ menu's Open in section), as data: the apps in the menu's
// order with the chosen one checked, and what a click on each does. The views draw it and the control folder drives it, so
// both go through these same calls.

import Foundation

@MainActor
public struct OpenInMenu {
    public struct Item: Equatable {
        public var app: OpenInApp
        public var checked: Bool
    }

    public let folder: String
    private let backend: any OpenInBackend
    public let choice: OpenInChoice
    public let items: [Item]

    public init(folder: String, backend: any OpenInBackend) {
        let choice = backend.choice()
        (self.folder, self.backend, self.choice) = (folder, backend, choice)
        items = backend.apps().map { Item(app: $0, checked: $0.bundle == choice.app.bundle) }
    }

    /// The button itself: the chosen app opens the folder.
    public func click() { backend.open(folder, in: choice.app) }

    /// An app of the list: it opens the folder (and becomes the chosen one, the window's to remember).
    public func pick(_ app: OpenInApp) { backend.open(folder, in: app) }
}
