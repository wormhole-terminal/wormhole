// The Project folders page's values as the core writes them (its sentences already in the language in use), and the
// seam the page acts through: the defaults every project starts from, the trusted list, and one project's trust and its
// own Default | On | Off for each feature.

public struct ProjectDefault: Sendable, Equatable, Identifiable {
    public var id: String { key }
    public let key: String
    public let label: String
    public let description: String
    public let on: Bool
    public init(key: String, label: String, description: String, on: Bool) {
        (self.key, self.label, self.description, self.on) = (key, label, description, on)
    }
}

/// A project the page can pick.
public struct ProjectChoice: Sendable, Equatable, Identifiable {
    public var id: String { root }
    public let root: String
    public let name: String
    /// Where it is, with `~` for the home folder.
    public let detail: String
    public init(root: String, name: String, detail: String) { (self.root, self.name, self.detail) = (root, name, detail) }
}

public struct ProjectFeatureRow: Sendable, Equatable, Identifiable {
    public var id: String { key }
    public let key: String
    public let label: String
    public let description: String
    /// The project has no such file: the row is faint.
    public let faint: Bool
    /// `nil` follows the default.
    public let choice: Bool?
    public let defaultOn: Bool
    public init(key: String, label: String, description: String, faint: Bool = false, choice: Bool? = nil, defaultOn: Bool = true) {
        (self.key, self.label, self.description, self.faint, self.choice, self.defaultOn) = (key, label, description, faint, choice, defaultOn)
    }
}

public struct ProjectPage: Sendable, Equatable {
    public enum Trust: Sendable { case trusted, changed, untrusted }
    public let root: String
    public let trust: Trust
    public let statusWord: String
    public let detail: String
    /// What it would run while it is not trusted: one line each (what, the command, a note).
    public let commands: [String]
    public let features: [ProjectFeatureRow]
    public let problems: [String]
    /// Why the last approval or save failed.
    public let error: String?
    /// An approval is waiting for the owner.
    public let approving: Bool
    /// `.wormhole` (or the older folder), for "Show .wormhole in Finder".
    public let dirName: String

    public init(
        root: String, trust: Trust, statusWord: String, detail: String, commands: [String] = [], features: [ProjectFeatureRow] = [],
        problems: [String] = [], error: String? = nil, approving: Bool = false, dirName: String = ".wormhole"
    ) {
        (self.root, self.trust, self.statusWord, self.detail, self.commands) = (root, trust, statusWord, detail, commands)
        (self.features, self.problems, self.error, self.approving, self.dirName) = (features, problems, error, approving, dirName)
    }
}

/// The core's project service behind the page. Reads touch the disk and approving waits for the owner: off the main thread.
public protocol ProjectsBackend: Sendable {
    func defaults() async -> [ProjectDefault]
    func setDefault(_ key: String, on: Bool) async
    func trusted() async -> [ProjectChoice]
    /// The projects worth offering: those of the open terminals' folders, then the trusted ones.
    func known() async -> [ProjectChoice]
    /// `nil`: the folder has no project folder anymore.
    func page(_ root: String) async -> ProjectPage?
    /// `nil` follows the default.
    func setChoice(_ root: String, feature: String, to choice: Bool?) async
    /// Asks the owner first (Touch ID or the password).
    func trust(_ root: String) async
    func revoke(_ root: String) async
    /// A new terminal in the project's folder (what only the window can do).
    @MainActor func openTerminal(_ root: String)
    /// Whether the owner can be asked at all (Touch ID or a login password); the page says so when not.
    @MainActor var canApprove: Bool { get }
    /// Calls `changed` once whenever the files of the project's `.wormhole` folder change, until the returned token is let go
    /// (the core's watch; a change arrives as an event, never by looking again and again).
    func follow(_ root: String, changed: @escaping @Sendable () -> Void) -> AnyObject?
}

extension ProjectsBackend {
    @MainActor public var canApprove: Bool { true }
    /// Nothing follows the folder by default (a backend with no watch): the page reads when the window comes forward.
    public func follow(_ root: String, changed: @escaping @Sendable () -> Void) -> AnyObject? { nil }
}
