// The apps a terminal's folder can be opened in from the title row's Open in… button, and which one a click
// opens (`platform/open_in.rs`, whose rule this is until it comes through the FFI). Whether an app is
// installed is the system's to say (`OpenInBackend`); this only holds the rule.

import Foundation

public struct OpenInApp: Sendable, Equatable, Hashable {
    /// The bundle identifier: how the app is found and launched.
    public var bundle: String
    public var name: String
    /// Two letters standing for the app when its own icon can't be had.
    public var monogram: String

    public init(bundle: String, name: String, monogram: String) { (self.bundle, self.name, self.monogram) = (bundle, name, monogram) }

    public static let finder = OpenInApp(bundle: "com.apple.finder", name: "Finder", monogram: "Fi") // text: Apple's app name and the two letters standing for it
}

/// What a click on Open in… opens, and the name of the chosen app when it is not installed.
public struct OpenInChoice: Sendable, Equatable {
    public var app: OpenInApp
    public var missing: String?

    public init(app: OpenInApp, missing: String? = nil) { (self.app, self.missing) = (app, missing) }

    /// The chosen app if installed, else the first editor found, else Finder. A chosen one that isn't
    /// installed is named so the tooltip can say so.
    public static func resolve(chosen: String, installed: [OpenInApp], knownName: (String) -> String?) -> OpenInChoice {
        let chosen = chosen.trimmingCharacters(in: .whitespaces)
        if let app = installed.first(where: { !chosen.isEmpty && $0.bundle == chosen }) { return OpenInChoice(app: app, missing: nil) }
        let app = installed.first { $0.bundle != OpenInApp.finder.bundle } ?? .finder
        return OpenInChoice(app: app, missing: chosen.isEmpty ? nil : (knownName(chosen) ?? chosen))
    }
}

/// The system's side of Open in…: which apps are installed, their icons, and opening a folder in one.
@MainActor
public protocol OpenInBackend {
    /// The apps to list, in the menu's order, Finder last.
    func apps() -> [OpenInApp]
    /// The app a click opens: the person's choice (an app-wide setting), or the fallback.
    func choice() -> OpenInChoice
    func open(_ folder: String, in app: OpenInApp)
}
